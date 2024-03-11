use super::FileDriver;
use rocket::{async_trait, fs::TempFile, tokio::fs::File};
use std::{fs::Metadata, path::PathBuf};
use uuid::Uuid;

pub struct LocalFileSystem {
    base_path: PathBuf,
    should_copy_files: bool,
}

impl LocalFileSystem {
    pub async fn new(
        base_path: impl Into<PathBuf>,
        temp_path: impl Into<PathBuf>,
    ) -> Result<Self, std::io::Error> {
        fn get_device_id(meta: &Metadata) -> Option<u64> {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                Some(meta.dev())
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                meta.volume_serial_number().map(|serial| serial as u64)
            }
            #[cfg(not(any(unix, windows)))]
            {
                None
            }
        }

        let base_path = base_path.into();
        let temp_path = temp_path.into();

        if !tokio::fs::try_exists(&base_path).await? {
            tokio::fs::create_dir_all(&base_path).await?;
        }

        if !tokio::fs::try_exists(&temp_path).await? {
            tokio::fs::create_dir_all(&temp_path).await?;
        }

        let base_path_meta = tokio::fs::metadata(&base_path).await?;
        let temp_path_meta = tokio::fs::metadata(&temp_path).await?;

        let base_path_device_id = get_device_id(&base_path_meta);
        let temp_path_device_id = get_device_id(&temp_path_meta);

        let should_copy_files = match (base_path_device_id, temp_path_device_id) {
            (Some(base_path_device_id), Some(temp_path_device_id)) => {
                base_path_device_id != temp_path_device_id
            }
            _ => true,
        };

        Ok(Self {
            base_path,
            should_copy_files,
        })
    }

    fn generate_local_file_path(&self, id: Uuid) -> PathBuf {
        self.base_path.join(id.to_string())
    }
}

#[async_trait]
impl FileDriver for LocalFileSystem {
    async fn commit(&self, id: Uuid, file: &mut TempFile) -> Result<(), std::io::Error> {
        let local_file_path = self.generate_local_file_path(id);

        if self.should_copy_files {
            file.copy_to(&local_file_path).await?;
        } else {
            file.persist_to(&local_file_path).await?;
        }

        Ok(())
    }

    async fn remove(&self, id: Uuid) -> Result<(), std::io::Error> {
        let local_file_path = self.generate_local_file_path(id);
        tokio::fs::remove_file(&local_file_path).await?;
        Ok(())
    }

    async fn read(&self, id: Uuid) -> Result<Option<File>, std::io::Error> {
        let local_file_path = self.generate_local_file_path(id);

        match File::open(&local_file_path).await {
            Ok(file) => Ok(Some(file)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }
}
