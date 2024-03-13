use super::{FileDriver, WriteError};
use crate::db::models::{CreatingStagingFile, StagingFile};
use chrono::{Duration, Utc};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::{
    pooled_connection::deadpool::Pool, scoped_futures::ScopedFutureExt, AsyncConnection,
    AsyncPgConnection, RunQueryDsl,
};
use rocket::data::DataStream;
use std::sync::Arc;
use thiserror::Error;
use tokio::task::JoinSet;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum StagingFileServiceError {
    #[error("database pool error: {0}")]
    PoolError(#[from] diesel_async::pooled_connection::deadpool::PoolError),
    #[error("diesel error: {0}")]
    DieselError(#[from] diesel::result::Error),
}

pub struct StagingFileService {
    db_pool: Pool<AsyncPgConnection>,
    file_driver: Arc<dyn FileDriver + Send + Sync>,
}

impl StagingFileService {
    pub fn new(
        db_pool: Pool<AsyncPgConnection>,
        file_driver: Arc<impl 'static + FileDriver + Send + Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            db_pool,
            file_driver,
        })
    }

    /// Creates a new staging file.
    pub async fn create_staging_file(
        &self,
        name: &str,
        mime: Option<&str>,
    ) -> Result<StagingFile, StagingFileServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let staging_file = diesel::insert_into(schema::staging_files::table)
            .values(CreatingStagingFile {
                name,
                mime,
                size: 0,
            })
            .returning((
                schema::staging_files::id,
                schema::staging_files::name,
                schema::staging_files::mime,
                schema::staging_files::size,
                schema::staging_files::staged_at,
            ))
            .get_result::<StagingFile>(db)
            .await?;

        Ok(staging_file)
    }

    /// Removes a staging file by its ID.
    /// Returns the staging file that was removed, or `None` if no staging file was found.
    /// The `db` parameter is a mutable reference to a database connection.
    /// This allows the caller to pass in a transaction, if needed.
    pub async fn remove_staging_file_by_id(
        &self,
        staging_file_id: Uuid,
        db: Option<&mut AsyncPgConnection>,
        delete_data: bool,
    ) -> Result<Option<StagingFile>, StagingFileServiceError> {
        use crate::db::schema;

        let mut fallback_db = match db {
            Some(_) => None,
            None => Some(self.db_pool.get().await?),
        };
        let db = match db {
            Some(db) => db,
            None => fallback_db.as_mut().unwrap(),
        };
        let staging_file = diesel::delete(
            schema::staging_files::dsl::staging_files
                .filter(schema::staging_files::id.eq(staging_file_id)),
        )
        .returning((
            schema::staging_files::id,
            schema::staging_files::name,
            schema::staging_files::mime,
            schema::staging_files::size,
            schema::staging_files::staged_at,
        ))
        .get_result::<StagingFile>(db)
        .await
        .optional()?;

        if staging_file.is_some() && delete_data {
            // it is safe to ignore the result of this operation
            self.file_driver.remove_staging(staging_file_id).await.ok();
        }

        Ok(staging_file)
    }

    /// Removes all expired staging files.
    /// Returns the number of staging files that were removed.
    /// Staging files are considered expired if they were staged more than `duration` ago.
    pub async fn remove_expired_staging_files(
        &self,
        duration: Duration,
        limit: u32,
    ) -> Result<usize, StagingFileServiceError> {
        use crate::db::schema;

        let now = Utc::now().naive_utc();
        let expiration_time = now - duration;

        let db = &mut self.db_pool.get().await?;
        let expired_staging_file_ids = schema::staging_files::dsl::staging_files
            .filter(schema::staging_files::staged_at.lt(expiration_time))
            .select(schema::staging_files::id)
            .order((
                schema::staging_files::staged_at.asc(),
                schema::staging_files::id.asc(),
            ))
            .limit(limit as i64)
            .load::<Uuid>(db)
            .await?;
        let expired_staging_files = diesel::delete(
            schema::staging_files::dsl::staging_files
                .filter(schema::staging_files::id.eq_any(expired_staging_file_ids)),
        )
        .returning(schema::staging_files::id)
        .get_results::<Uuid>(db)
        .await?;

        let mut removal_tasks = JoinSet::new();

        async fn remove_staging_file(
            file_driver: Arc<dyn FileDriver + Send + Sync>,
            staging_file_id: Uuid,
        ) {
            file_driver.remove_staging(staging_file_id).await.ok();
        }

        for staging_file_id in &expired_staging_files {
            let file_driver = self.file_driver.clone();
            let task = remove_staging_file(file_driver, *staging_file_id);
            removal_tasks.spawn_local(task);
        }

        removal_tasks.detach_all();

        Ok(expired_staging_files.len())
    }

    /// Retrieves a staging file by its ID.
    pub async fn get_staging_file_by_id(
        &self,
        staging_file_id: Uuid,
    ) -> Result<Option<StagingFile>, StagingFileServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let staging_file = schema::staging_files::dsl::staging_files
            .filter(schema::staging_files::id.eq(staging_file_id))
            .select((
                schema::staging_files::id,
                schema::staging_files::name,
                schema::staging_files::mime,
                schema::staging_files::size,
                schema::staging_files::staged_at,
            ))
            .get_result::<StagingFile>(db)
            .await
            .optional()?;

        Ok(staging_file)
    }

    /// Fills a staging file by its ID.
    /// Returns the updated staging file, or `None` if no staging file was found.
    /// It will lock the staging file for writing, so that no other operation can write to it at the same time.
    pub async fn fill_staging_file_by_id(
        &self,
        staging_file_id: Uuid,
        offset: Option<u64>,
        stream: DataStream<'_>,
    ) -> Result<Result<Option<StagingFile>, WriteError>, StagingFileServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        db.transaction(|db| {
            async move {
                let staging_file_id = schema::staging_files::dsl::staging_files
                    .filter(schema::staging_files::id.eq(staging_file_id))
                    .select(schema::staging_files::id)
                    .for_update()
                    .get_result::<Uuid>(db)
                    .await
                    .optional()?;
                let staging_file_id = match staging_file_id {
                    Some(staging_file_id) => staging_file_id,
                    None => {
                        return Ok(Ok(None));
                    }
                };

                let result = self
                    .file_driver
                    .write_staging(staging_file_id, offset.unwrap_or(0), stream)
                    .await;
                let size = match result {
                    Ok(size) => size,
                    Err(err) => {
                        return Ok(Err(err));
                    }
                };

                let staging_file = diesel::update(
                    schema::staging_files::dsl::staging_files
                        .filter(schema::staging_files::id.eq(staging_file_id)),
                )
                .set(schema::staging_files::size.eq(size))
                .returning((
                    schema::staging_files::id,
                    schema::staging_files::name,
                    schema::staging_files::mime,
                    schema::staging_files::size,
                    schema::staging_files::staged_at,
                ))
                .get_result::<StagingFile>(db)
                .await?;

                Ok(Ok(Some(staging_file)))
            }
            .scope_boxed()
        })
        .await
    }
}
