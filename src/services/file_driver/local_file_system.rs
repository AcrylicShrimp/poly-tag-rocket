use super::{FileDriver, ReadError, ReadRange, WriteError};
use rocket::{async_trait, data::DataStream, tokio::fs::File};
use std::{fs::Metadata, path::PathBuf, pin::Pin};
use tokio::{
    fs::OpenOptions,
    io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, SeekFrom},
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

        let staging_path_exists = tokio::fs::try_exists(&staging_path).await;
        let staging_path_exists = match staging_path_exists {
            Ok(exists) => exists,
            Err(err) => {
                log::error!(target: "file_driver", method="new", staging_path:?, resident_path:?, err:err; "Failed to check if staging path exists.");
                return Err(err);
            }
        };

        let resident_path_exists = tokio::fs::try_exists(&resident_path).await;
        let resident_path_exists = match resident_path_exists {
            Ok(exists) => exists,
            Err(err) => {
                log::error!(target: "file_driver", method="new", staging_path:?, resident_path:?, err:err; "Failed to check if resident path exists.");
                return Err(err);
            }
        };

        if !staging_path_exists {
            if let Err(err) = tokio::fs::create_dir_all(&staging_path).await {
                log::error!(target: "file_driver", method="new", staging_path:?, resident_path:?, err:err; "Failed to create staging path.");
                return Err(err);
            }
        }

        if !resident_path_exists {
            if let Err(err) = tokio::fs::create_dir_all(&resident_path).await {
                log::error!(target: "file_driver", method="new", staging_path:?, resident_path:?, err:err; "Failed to create resident path.");
                return Err(err);
            }
        }

        let staging_path_meta = tokio::fs::metadata(&staging_path).await;
        let staging_path_meta = match staging_path_meta {
            Ok(meta) => meta,
            Err(err) => {
                log::error!(target: "file_driver", method="new", staging_path:?, resident_path:?, err:err; "Failed to get metadata of staging path.");
                return Err(err);
            }
        };

        let resident_path_meta = tokio::fs::metadata(&resident_path).await;
        let resident_path_meta = match resident_path_meta {
            Ok(meta) => meta,
            Err(err) => {
                log::error!(target: "file_driver", method="new", staging_path:?, resident_path:?, err:err; "Failed to get metadata of resident path.");
                return Err(err);
            }
        };

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
            WriteError::Write {
                io_error,
                file_size,
            }
        }

        let path = self.generate_staging_file_path(id);

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .await;
        let mut file = match file {
            Ok(file) => file,
            Err(err) => {
                log::error!(target: "file_driver", method="write_staging", id:serde, path:?, err:err; "Failed to open file.");
                return Err(make_write_error(err, 0));
            }
        };

        let initial_file_size = file.metadata().await.map(|meta| meta.len());
        let initial_file_size = match initial_file_size {
            Ok(size) => size,
            Err(err) => {
                log::error!(target: "file_driver", method="write_staging", id:serde, path:?, err:err; "Failed to get file size.");
                return Err(make_write_error(err, 0));
            }
        };

        if (i64::MAX as u64) < initial_file_size {
            return Err(WriteError::FileTooLarge {
                max_size: i64::MAX as u64,
                file_size: initial_file_size,
            });
        }

        if initial_file_size < offset {
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

        if let Err(err) = file.seek(SeekFrom::Start(offset)).await {
            log::error!(target: "file_driver", method="write_staging", id:serde, path:?, err:err; "Failed to seek file.");
            return Err(make_write_error(err, initial_file_size));
        }

        let copy_result = tokio::io::copy(&mut stream, &mut file).await;
        let copy_err = match copy_result {
            Ok(_) => None,
            Err(err) => {
                log::error!(target: "file_driver", method="write_staging", id:serde, path:?, err:err; "Failed to write to file.");
                Some(err)
            }
        };

        if let Err(err) = file.flush().await {
            log::error!(target: "file_driver", method="write_staging", id:serde, path:?, err:err; "Failed to flush file.");
            // here, we don't return the error, because we must carry on.
        }

        let file_size = file.metadata().await.map(|meta| meta.len());
        let file_size = match file_size {
            Ok(size) => size,
            Err(err) => {
                log::error!(target: "file_driver", method="write_staging", id:serde, path:?, err:err; "Failed to get file size after write.");
                // assume the write operation has been failed entirely, since we can't get the file size.
                initial_file_size
            }
        };

        match copy_err {
            Some(err) => Err(make_write_error(err, file_size)),
            None => Ok(file_size as i64),
        }
    }

    async fn remove_staging(&self, id: Uuid) -> Result<(), std::io::Error> {
        let path = self.generate_staging_file_path(id);

        if let Err(err) = tokio::fs::remove_file(&path).await {
            log::error!(target: "file_driver", method="remove_staging", id:serde, path:?, err:err; "Failed to remove file.");
            return Err(err);
        }

        Ok(())
    }

    async fn read_staging(&self, id: Uuid) -> Result<Option<PathBuf>, std::io::Error> {
        let path = self.generate_staging_file_path(id);

        match File::open(&path).await {
            Ok(_) => Ok(Some(path)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => {
                log::error!(target: "file_driver", method="read_staging", id:serde, path:?, err:err; "Failed to open file.");
                Err(err)
            }
        }
    }

    async fn commit_staging(&self, id: Uuid) -> Result<(), std::io::Error> {
        let staging_path = self.generate_staging_file_path(id);
        let resident_path = self.generate_resident_file_path(id);

        if self.should_copy_files {
            if let Err(err) = tokio::fs::copy(&staging_path, &resident_path).await {
                log::error!(target: "file_driver", method="commit_staging", id:serde, staging_path:?, resident_path:?, err:err; "Failed to copy file.");
                return Err(err);
            }

            if let Err(err) = tokio::fs::remove_file(&staging_path).await {
                log::warn!(target: "file_driver", method="commit_staging", id:serde, staging_path:?, resident_path:?, err:err; "Failed to remove file.");
                // removing the staging file is not critical, so we don't return the error.
            }
        } else if let Err(err) = tokio::fs::rename(&staging_path, &resident_path).await {
            log::error!(target: "file_driver", method="commit_staging", id:serde, staging_path:?, resident_path:?, err:err; "Failed to rename file.");
            return Err(err);
        }

        Ok(())
    }

    async fn remove(&self, id: Uuid) -> Result<(), std::io::Error> {
        let path = self.generate_resident_file_path(id);

        if let Err(err) = tokio::fs::remove_file(&path).await {
            log::error!(target: "file_driver", method="remove", id:serde, path:?, err:err; "Failed to remove file.");
            return Err(err);
        }

        Ok(())
    }

    async fn read(
        &self,
        id: Uuid,
        read_range: ReadRange,
    ) -> Result<Option<Pin<Box<dyn AsyncRead + Send>>>, ReadError> {
        let path = self.generate_resident_file_path(id);

        let mut file = match File::open(&path).await {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(err) => {
                log::error!(target: "file_driver", method="read", id:serde, path:?, err:err; "Failed to open file.");
                return Err(ReadError::Read { io_error: err });
            }
        };
        let file_size = file.metadata().await.map(|meta| meta.len());
        let file_size = match file_size {
            Ok(file_size) => file_size,
            Err(err) => {
                log::error!(target: "file_driver", method="read", id:serde, path:?, err:err; "Failed to get file size.");
                return Err(ReadError::Read { io_error: err });
            }
        };

        let reader: Pin<Box<dyn AsyncRead + Send>> = match read_range {
            ReadRange::Full => Box::pin(BufReader::new(file)),
            ReadRange::Start(start) => {
                if file_size <= start {
                    return Err(ReadError::RangeStartExceedsFileSize { start, file_size });
                }

                if let Err(err) = file.seek(SeekFrom::Start(start)).await {
                    log::error!(target: "file_driver", method="read", id:serde, path:?, file_size, start, err:err; "Failed to seek file.");
                    return Err(ReadError::Read { io_error: err });
                }

                Box::pin(BufReader::new(file))
            }
            ReadRange::Range(start, end) => {
                if file_size <= end {
                    return Err(ReadError::RangeEndExceedsFileSize { end, file_size });
                }

                if let Err(err) = file.seek(SeekFrom::Start(start)).await {
                    log::error!(target: "file_driver", method="read", id:serde, path:?, file_size, start, end, err:err; "Failed to seek file.");
                    return Err(ReadError::Read { io_error: err });
                }

                Box::pin(BufReader::new(file.take(end - start + 1)))
            }
            ReadRange::Suffix(suffix) => {
                // it is allowed to specify a suffix that is larger than the file size.
                // in that case, we just read the entire file instead.
                let suffix = (suffix as u64).min(file_size);

                if let Err(err) = file.seek(SeekFrom::End(-(suffix as i64))).await {
                    log::error!(target: "file_driver", method="read", id:serde, path:?, file_size, suffix, err:err; "Failed to seek file.");
                    return Err(ReadError::Read { io_error: err });
                }

                Box::pin(BufReader::new(file))
            }
        };

        Ok(Some(reader))
    }
}
