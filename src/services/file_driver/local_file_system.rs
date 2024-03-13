use super::{FileDriver, WriteError};
use rocket::{async_trait, data::DataStream, tokio::fs::File};
use std::{fs::Metadata, path::PathBuf, pin::Pin};
use tokio::{
    fs::OpenOptions,
    io::{AsyncRead, AsyncSeekExt, AsyncWriteExt, BufReader, SeekFrom},
};
use uuid::Uuid;

pub struct LocalFileSystem {
    staging_path: PathBuf,
    resident_path: PathBuf,
    should_copy_files: bool,
}

impl LocalFileSystem {
    pub async fn new(
        staging_path: impl Into<PathBuf>,
        resident_path: impl Into<PathBuf>,
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

        let staging_path = staging_path.into();
        let resident_path = resident_path.into();

        if !tokio::fs::try_exists(&staging_path).await? {
            tokio::fs::create_dir_all(&staging_path).await?;
        }

        if !tokio::fs::try_exists(&resident_path).await? {
            tokio::fs::create_dir_all(&resident_path).await?;
        }

        let staging_path_meta = tokio::fs::metadata(&staging_path).await?;
        let resident_path_meta = tokio::fs::metadata(&resident_path).await?;

        let staging_path_device_id = get_device_id(&staging_path_meta);
        let resident_path_device_id = get_device_id(&resident_path_meta);

        let should_copy_files = match (staging_path_device_id, resident_path_device_id) {
            (Some(staging_path_device_id), Some(resident_path_device_id)) => {
                staging_path_device_id != resident_path_device_id
            }
            _ => true,
        };

        Ok(Self {
            staging_path,
            resident_path,
            should_copy_files,
        })
    }

    fn generate_staging_file_path(&self, id: Uuid) -> PathBuf {
        self.staging_path.join(id.to_string())
    }

    fn generate_resident_file_path(&self, id: Uuid) -> PathBuf {
        self.resident_path.join(id.to_string())
    }
}

#[async_trait]
impl FileDriver for LocalFileSystem {
    async fn write_staging(
        &self,
        id: Uuid,
        offset: u64,
        mut stream: DataStream<'_>,
    ) -> Result<i64, WriteError> {
        fn make_write_error(io_error: std::io::Error, file_size: u64) -> WriteError {
            WriteError::WriteError {
                io_error,
                file_size,
            }
        }

        let path = self.generate_staging_file_path(id);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&path)
            .await
            .map_err(|err| make_write_error(err, 0))?;
        let initial_file_size = file
            .metadata()
            .await
            .map(|meta| meta.len())
            .map_err(|err| make_write_error(err, 0))?;

        if (i64::MAX as u64) < initial_file_size {
            return Err(WriteError::FileTooLarge {
                max_size: i64::MAX as u64,
                file_size: initial_file_size,
            });
        }

        if initial_file_size < offset as u64 {
            return Err(WriteError::OffsetExceedsFileSize {
                offset,
                file_size: initial_file_size,
            });
        }

        if (i64::MAX as u128) < offset as u128 + initial_file_size as u128 {
            return Err(WriteError::OffsetTooLarge {
                max_offset: i64::MAX as u64,
                offset,
            });
        }

        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|err| make_write_error(err, initial_file_size))?;

        let copy_result = tokio::io::copy(&mut stream, &mut file).await;
        file.flush().await.ok();

        let file_size = file
            .metadata()
            .await
            .map(|meta| meta.len())
            .ok()
            .unwrap_or_else(|| initial_file_size);

        match copy_result {
            Ok(_) => Ok(file_size as i64),
            Err(err) => Err(make_write_error(err, file_size)),
        }
    }

    async fn remove_staging(&self, id: Uuid) -> Result<(), std::io::Error> {
        let path = self.generate_staging_file_path(id);
        tokio::fs::remove_file(&path).await?;

        Ok(())
    }

    async fn read_staging(&self, id: Uuid) -> Result<Option<PathBuf>, std::io::Error> {
        let path = self.generate_staging_file_path(id);

        match File::open(&path).await {
            Ok(_) => Ok(Some(path)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

    async fn commit_staging(&self, id: Uuid) -> Result<(), std::io::Error> {
        let staging_path = self.generate_staging_file_path(id);
        let resident_path = self.generate_resident_file_path(id);

        if self.should_copy_files {
            tokio::fs::copy(&staging_path, &resident_path).await?;
            tokio::fs::remove_file(&staging_path).await?;
        } else {
            tokio::fs::rename(&staging_path, &resident_path).await?;
        }

        Ok(())
    }

    async fn remove(&self, id: Uuid) -> Result<(), std::io::Error> {
        let path = self.generate_resident_file_path(id);
        tokio::fs::remove_file(&path).await?;

        Ok(())
    }

    async fn read(
        &self,
        id: Uuid,
    ) -> Result<Option<Pin<Box<dyn AsyncRead + Send>>>, std::io::Error> {
        let path = self.generate_resident_file_path(id);

        match File::open(&path).await {
            Ok(file) => Ok(Some(Box::pin(BufReader::new(file)))),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }
}
