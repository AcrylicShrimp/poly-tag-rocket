mod compute_file_hash;
mod compute_file_mime;

use super::{FileDriver, StagingFileService, StagingFileServiceError};
use crate::db::models::{CreatingFile, File};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
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
    PoolError(#[from] diesel_async::pooled_connection::deadpool::PoolError),
    #[error("diesel error: {0}")]
    DieselError(#[from] diesel::result::Error),
    #[error("staging file service error: {0}")]
    StagingFileServiceError(#[from] StagingFileServiceError),
    #[error("file is not yet filled; upload it first")]
    FileNotYetFilled,
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("compute file mime error: {0}")]
    ComputeMimeError(#[from] compute_file_mime::ComputeFileMimeError),
    #[error("compute file hash error: {0}")]
    ComputeHashError(#[from] compute_file_hash::ComputeFileHashError),
}

pub struct FileService {
    db_pool: Pool<AsyncPgConnection>,
    staging_file_service: Arc<StagingFileService>,
    file_driver: Arc<dyn FileDriver + Send + Sync>,
}

impl FileService {
    pub fn new(
        db_pool: Pool<AsyncPgConnection>,
        staging_file_service: Arc<StagingFileService>,
        file_driver: Arc<impl 'static + FileDriver + Send + Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            db_pool,
            staging_file_service,
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
                        Some(mime) => Ok(Some(mime.as_str())),
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
                let mime = mime.unwrap_or("application/octet-stream");

                let file = diesel::insert_into(schema::files::table)
                    .values(CreatingFile {
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
        }

        Ok(file)
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
    ) -> Result<Option<Pin<Box<dyn AsyncRead + Send>>>, FileServiceError> {
        let data = self.file_driver.read(file_id).await?;

        Ok(data)
    }
}
