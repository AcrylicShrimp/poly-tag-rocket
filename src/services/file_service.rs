mod compute_file_hash;
mod compute_file_mime;

use super::{
    FileDriver, ReadError, ReadRange, SearchService, StagingFileService, StagingFileServiceError,
};
use crate::db::models::{CreatingFile, File};
use diesel::{BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::{
    pooled_connection::deadpool::Pool, scoped_futures::ScopedFutureExt, AsyncConnection,
    AsyncPgConnection, RunQueryDsl,
};
use std::{pin::Pin, sync::Arc};
use thiserror::Error;
use tokio::io::AsyncRead;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum FileServiceError {
    #[error("database pool error: {0}")]
    Pool(#[from] diesel_async::pooled_connection::deadpool::PoolError),
    #[error("diesel error: {0}")]
    Diesel(#[from] diesel::result::Error),
    #[error("staging file service error: {0}")]
    StagingFileService(#[from] StagingFileServiceError),
    #[error("file is not yet filled; upload it first")]
    FileNotYetFilled,
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
    #[error("compute file mime error: {0}")]
    ComputeMime(#[from] compute_file_mime::ComputeFileMimeError),
    #[error("compute file hash error: {0}")]
    ComputeHash(#[from] compute_file_hash::ComputeFileHashError),
}

pub struct FileService {
    db_pool: Pool<AsyncPgConnection>,
    staging_file_service: Arc<StagingFileService>,
    search_service: Arc<SearchService>,
    file_driver: Arc<dyn FileDriver + Send + Sync>,
}

impl FileService {
    pub fn new(
        db_pool: Pool<AsyncPgConnection>,
        staging_file_service: Arc<StagingFileService>,
        search_service: Arc<SearchService>,
        file_driver: Arc<impl 'static + FileDriver + Send + Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            db_pool,
            staging_file_service,
            search_service,
            file_driver,
        })
    }

    /// Creates a new file from a staging file.
    /// It computes the file's MIME type and hash, and stores the file in the file driver.
    pub async fn create_file_from_staging_file_id(
        &self,
        staging_file_id: Uuid,
    ) -> Result<Option<File>, FileServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        db.transaction(|db| {
            async move {
                let staging_file = self
                    .staging_file_service
                    .remove_staging_file_by_id(staging_file_id, Some(db), false)
                    .await?;

                let staging_file = match staging_file {
                    Some(staging_file) => staging_file,
                    None => {
                        return Ok(None);
                    }
                };

                let file = self.file_driver.read_staging(staging_file.id).await?;
                let file_path = match file {
                    Some(file) => file,
                    None => {
                        return Err(FileServiceError::FileNotYetFilled);
                    }
                };

                let compute_mime = || async {
                    match &staging_file.mime {
                        Some(mime) => Ok(mime.as_str()),
                        None => compute_file_mime::compute_file_mime(&file_path)
                            .await
                            .map_err(FileServiceError::from),
                    }
                };
                let compute_hash = || async {
                    compute_file_hash::compute_file_hash(&file_path)
                        .await
                        .map_err(FileServiceError::from)
                };

                let size = tokio::fs::metadata(&file_path).await?.len();
                let (mime, hash) = tokio::try_join!(compute_mime(), compute_hash())?;

                let file = diesel::insert_into(schema::files::table)
                    .values(CreatingFile {
                        id: staging_file.id,
                        name: &staging_file.name,
                        mime,
                        size: size as i64,
                        hash: hash as i64,
                    })
                    .returning((
                        schema::files::id,
                        schema::files::name,
                        schema::files::mime,
                        schema::files::size,
                        schema::files::hash,
                        schema::files::uploaded_at,
                    ))
                    .get_result::<File>(db)
                    .await?;

                self.file_driver.commit_staging(staging_file.id).await?;

                // ignore the error if the indexing fails, as it is not critical
                self.search_service.index_file(&file).await.ok();

                Ok(Some(file))
            }
            .scope_boxed()
        })
        .await
    }

    /// Removes a file by its ID.
    /// Returns the file that was removed, or `None` if no file was found.
    /// It also removes the file from the file driver.
    pub async fn remove_file_by_id(&self, file_id: Uuid) -> Result<Option<File>, FileServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let file = diesel::delete(
            crate::db::schema::files::table.filter(crate::db::schema::files::id.eq(file_id)),
        )
        .returning((
            schema::files::id,
            schema::files::name,
            schema::files::mime,
            schema::files::size,
            schema::files::hash,
            schema::files::uploaded_at,
        ))
        .get_result::<File>(db)
        .await
        .optional()?;

        if file.is_some() {
            // it is safe to ignore the result of this operation
            self.file_driver.remove(file_id).await.ok();

            // ignore the error if the indexing fails, as it is not critical
            self.search_service.remove_file_by_id(file_id).await.ok();
        }

        Ok(file)
    }

    /// Retrieves a list of files.
    /// The result will be sorted by name and ID (name first) in ascending order.
    /// If `last_file_id` is provided, the result will start from the file that comes after it.
    pub async fn get_files(
        &self,
        last_file_id: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<File>, FileServiceError> {
        use crate::db::schema;
        let db = &mut self.db_pool.get().await?;

        let query = schema::files::dsl::files
            .select((
                schema::files::id,
                schema::files::name,
                schema::files::mime,
                schema::files::size,
                schema::files::hash,
                schema::files::uploaded_at,
            ))
            .order((schema::files::name.asc(), schema::files::id.asc()))
            .limit(limit as i64);

        let last_file = match last_file_id {
            Some(last_file_id) => {
                let last_file = schema::files::dsl::files
                    .select((schema::files::name, schema::files::id))
                    .filter(schema::files::id.eq(last_file_id))
                    .get_result::<(String, Uuid)>(db)
                    .await
                    .optional()?;

                let last_file = match last_file {
                    Some(pair) => pair,
                    None => return Ok(Vec::new()),
                };

                Some(last_file)
            }
            None => None,
        };

        let files = match &last_file {
            Some((last_file_name, last_file_id)) => query
                .filter(
                    schema::files::name
                        .gt(last_file_name)
                        .or(schema::files::name
                            .eq(last_file_name)
                            .and(schema::files::id.gt(last_file_id))),
                )
                .load::<File>(db),
            None => query.load::<File>(db),
        };
        let files = files.await?;

        Ok(files)
    }

    /// Retrieves a file by its ID.
    pub async fn get_file_by_id(&self, file_id: Uuid) -> Result<Option<File>, FileServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let file = schema::files::table
            .filter(schema::files::id.eq(file_id))
            .select((
                schema::files::id,
                schema::files::name,
                schema::files::mime,
                schema::files::size,
                schema::files::hash,
                schema::files::uploaded_at,
            ))
            .get_result::<File>(db)
            .await
            .optional()?;

        Ok(file)
    }

    /// Retrieves the file data by its ID.
    pub async fn get_file_data_by_id(
        &self,
        file_id: Uuid,
        range: ReadRange,
    ) -> Result<Option<Pin<Box<dyn AsyncRead + Send>>>, ReadError> {
        let data = self.file_driver.read(file_id, range).await?;

        Ok(data)
    }
}
