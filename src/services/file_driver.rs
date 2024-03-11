pub mod local_file_system;

use rocket::{async_trait, fs::TempFile, tokio::fs::File};
use uuid::Uuid;

#[async_trait]
pub trait FileDriver {
    /// Commits a file to the storage system.
    /// The committed file must be uniquely identified by the given `id`.
    async fn commit(&self, id: Uuid, file: &mut TempFile) -> Result<(), std::io::Error>;

    /// Removes a file from the storage system.
    async fn remove(&self, id: Uuid) -> Result<(), std::io::Error>;

    /// Reads a file from the storage system.
    /// Returns the file if it exists, otherwise `None`.
    /// It must return a local file. In case of a remote storage system, the file must be downloaded before returning it.
    /// It is guaranteed that the file will only be read; it will not be modified, moved or deleted.
    async fn read(&self, id: Uuid) -> Result<Option<File>, std::io::Error>;
}
