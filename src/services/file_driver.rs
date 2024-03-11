pub mod local_file_system;

use rocket::{fs::TempFile, tokio::fs::File};
use uuid::Uuid;

pub trait FileDriver: Sized {
    type Error;

    /// Commits a file to the storage system.
    /// The committed file must be uniquely identified by the given `id`.
    fn commit(&self, id: Uuid, file: &mut TempFile) -> Result<(), Self::Error>;

    /// Removes a file from the storage system.
    /// Returns the removed file's id if it existed, otherwise `None`.
    fn remove(&self, id: Uuid) -> Result<Option<Uuid>, Self::Error>;

    /// Reads a file from the storage system.
    /// Returns the file if it exists, otherwise `None`.
    /// It must return a local file. In case of a remote storage system, the file must be downloaded before returning it.
    /// It is guaranteed that the file will only be read; it will not be modified, moved or deleted.
    fn read(&self, id: Uuid) -> Result<Option<File>, Self::Error>;
}
