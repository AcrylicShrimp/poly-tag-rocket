use std::{io::Error as IOError, path::PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ComputeFileMimeError {
    #[error("failed to infer mime: {0}")]
    Infer(IOError),
    #[error("failed to join task: {0}")]
    Join(#[from] tokio::task::JoinError),
}

pub async fn compute_file_mime(
    path: impl Into<PathBuf>,
) -> Result<&'static str, ComputeFileMimeError> {
    let path = path.into();

    tokio::task::spawn_blocking(move || {
        let mime = infer::get_from_path(&path).map_err(ComputeFileMimeError::Infer)?;

        let mime = mime
            .map(|mime| mime.mime_type())
            .or_else(|| mime_guess::from_path(&path).first_raw())
            .unwrap_or("application/octet-stream");

        Ok(mime)
    })
    .await?
}
