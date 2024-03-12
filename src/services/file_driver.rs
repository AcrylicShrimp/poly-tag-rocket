pub mod local_file_system;

use async_trait::async_trait;
use rocket::data::DataStream;
use std::path::PathBuf;
use thiserror::Error;
use tokio::io::AsyncRead;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum WriteError {
    /// The offset exceeds the file size.
    #[error("offset exceeds file size: {file_size} < {offset}")]
    OffsetExceedsFileSize { offset: u64, file_size: u64 },
    /// An I/O error occurred while writing the file.
    #[error("IO error: {io_error}")]
    WriteError {
        io_error: std::io::Error,
        file_size: u64,
    },
    /// The file size is larger than the maximum allowed value.
    /// This error will be emitted if the blow condition is met:
    ///
    /// `i64::MAX` < `file_size`
    #[error("file size is larger than the maximum allowed value: {max_size} < {file_size}")]
    FileTooLarge { max_size: u64, file_size: u64 },
    /// The offset is larger than the maximum allowed value.
    /// This error will be emitted if the blow condition is met:
    ///
    /// `i64::MAX` < `offset` + `file_size`
    #[error("offset is larger than the maximum allowed value: {max_offset} < {offset}")]
    OffsetTooLarge { max_offset: u64, offset: u64 },
}

#[async_trait]
pub trait FileDriver {
    /// Writes data to a staging file in the storage system.
    /// The file must be uniquely identified by the given `id`.
    /// It must to keep the file in local storage until it is committed, since it may be written multiple times.
    /// `offset` is the position in the file where the data should be written. It is used to support resuming uploads.
    ///
    /// ## Error handling
    ///
    /// The file should be consistent and readable even if the write operation fails.
    async fn write_staging(
        &self,
        id: Uuid,
        offset: u64,
        stream: DataStream<'_>,
    ) -> Result<i64, WriteError>;

    /// Removes a staging file from the storage system.
    async fn remove_staging(&self, id: Uuid) -> Result<(), std::io::Error>;

    /// Reads a staging file from the storage system.
    /// Returns the file if it exists, otherwise `None`.
    async fn read_staging(&self, id: Uuid) -> Result<Option<PathBuf>, std::io::Error>;

    /// Commits a staging file to the storage system.
    /// The file must be uniquely identified by the given `id`.
    /// In case of a remote storage system, the file must be uploaded by this method.
    async fn commit_staging(&self, id: Uuid) -> Result<(), std::io::Error>;

    /// Removes a file from the storage system.
    async fn remove(&self, id: Uuid) -> Result<(), std::io::Error>;

    /// Reads a file from the storage system.
    /// Returns the file if it exists, otherwise `None`.
    async fn read(&self, id: Uuid) -> Result<Option<Box<dyn AsyncRead>>, std::io::Error>;
}
