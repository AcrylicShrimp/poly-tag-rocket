use std::{io::Error as IOError, path::PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ComputeFileMimeError {
    #[error("failed to infer mime: {0}")]
    InferError(IOError),
    #[error("failed to join task: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}

pub async fn compute_file_mime(
    path: impl Into<PathBuf>,
) -> Result<Option<&'static str>, ComputeFileMimeError> {
    let path = path.into();

    tokio::task::spawn_blocking(move || {
        let mime =
            infer::get_from_path(&path).map_err(|err| ComputeFileMimeError::InferError(err))?;

        match mime {
            Some(mime) => return Ok(Some(mime.mime_type())),
            None => Ok(Some(
                mime_guess::from_path(&path)
                    .first_raw()
                    .unwrap_or_else(|| "application/octet-stream"),
            )),
        }
    })
    .await?
}
