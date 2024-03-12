use crate::db::models::{CreatingStagingFile, StagingFile};
use chrono::{Duration, Utc};
use diesel::{query_dsl::methods::LockingDsl, ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::{
    pooled_connection::deadpool::Pool, scoped_futures::ScopedFutureExt, AsyncConnection,
    AsyncPgConnection, RunQueryDsl,
};
use rocket::fs::TempFile;
use std::sync::Arc;
use thiserror::Error;
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
}

impl StagingFileService {
    pub fn new(db_pool: Pool<AsyncPgConnection>) -> Arc<Self> {
        Arc::new(Self { db_pool })
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

    pub async fn fill_staging_file_by_id(
        &self,
        staging_file_id: Uuid,
        mut temp_file: TempFile<'_>,
        offset: Option<u64>,
    ) -> Result<Option<StagingFile>, StagingFileServiceError> {
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
                        return Ok(None);
                    }
                };

                async fn write_file_somewhere(
                    id: Uuid,
                    temp_file: &mut TempFile<'_>,
                    offset: Option<u64>,
                ) -> Result<i64, StagingFileServiceError> {
                    todo!()
                }

                let size: i64 =
                    write_file_somewhere(staging_file_id, &mut temp_file, offset).await?;
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

                Ok(Some(staging_file))
            }
            .scope_boxed()
        })
        .await
    }

    /// Removes a staging file by its ID.
    /// Returns the staging file that was removed, or `None` if no staging file was found.
    /// The `db` parameter is a mutable reference to a database connection.
    /// This allows the caller to pass in a transaction, if needed.
    pub async fn remove_staging_file_by_id(
        &self,
        staging_file_id: Uuid,
        db: Option<&mut AsyncPgConnection>,
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

        Ok(staging_file)
    }

    /// Removes all expired staging files.
    /// Returns the number of staging files that were removed.
    /// Staging files are considered expired if they were staged more than `duration` ago.
    pub async fn remove_expired_staging_files(
        &self,
        duration: Duration,
    ) -> Result<usize, StagingFileServiceError> {
        use crate::db::schema;

        let now = Utc::now().naive_utc();
        let expiration_time = now - duration;

        let db = &mut self.db_pool.get().await?;
        let expired_staging_files = diesel::delete(
            schema::staging_files::dsl::staging_files
                .filter(schema::staging_files::staged_at.lt(expiration_time)),
        )
        .execute(db)
        .await?;

        Ok(expired_staging_files)
    }
}
