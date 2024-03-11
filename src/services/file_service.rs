mod compute_file_hash;
mod compute_file_mime;

use super::file_driver::FileDriver;
use crate::db::models::{CreatingFile, File};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};
use rocket::fs::TempFile;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileServiceError {
    #[error("database pool error: {0}")]
    PoolError(#[from] diesel_async::pooled_connection::deadpool::PoolError),
    #[error("diesel error: {0}")]
    DieselError(#[from] diesel::result::Error),
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("compute file mime error: {0}")]
    ComputeMimeError(#[from] compute_file_mime::ComputeFileMimeError),
    #[error("compute file hash error: {0}")]
    ComputeHashError(#[from] compute_file_hash::ComputeFileHashError),
}

pub struct FileService {
    db_pool: Pool<AsyncPgConnection>,
    file_driver: Box<dyn FileDriver + Send + Sync>,
}

impl FileService {
    pub fn new(
        db_pool: Pool<AsyncPgConnection>,
        file_driver: Box<impl 'static + FileDriver + Send + Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            db_pool,
            file_driver: file_driver as Box<dyn FileDriver + Send + Sync>,
        })
    }

    pub async fn create_file(
        &self,
        name: &str,
        mime: Option<&str>,
        mut temp_file: TempFile<'_>,
    ) -> Result<File, FileServiceError> {
        use crate::db::schema;

        let compute_mime = || async {
            match mime {
                Some(mime) => Ok(Some(mime)),
                None => compute_file_mime::compute_file_mime(&temp_file)
                    .await
                    .map_err(FileServiceError::from),
            }
        };
        let compute_hash = || async {
            compute_file_hash::compute_file_hash(&temp_file)
                .await
                .map_err(FileServiceError::from)
        };

        let size = temp_file.len();
        let (mime, hash) = tokio::try_join!(compute_mime(), compute_hash(),)?;
        let mime = mime.unwrap_or("application/octet-stream");

        let db = &mut self.db_pool.get().await?;
        let file = diesel::insert_into(schema::files::table)
            .values(CreatingFile {
                name,
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
                schema::files::created_at,
            ))
            .get_result::<File>(db)
            .await?;

        self.file_driver.commit(file.id, &mut temp_file).await?;

        Ok(file)
    }
}
