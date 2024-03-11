use either::Either;
use rocket::fs::TempFile;
use std::io::Error as IOError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ComputeFileMimeError {
    #[error("failed to infer mime: {0}")]
    InferError(IOError),
    #[error("failed to join task: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}

pub async fn compute_file_mime(
    file: &TempFile<'_>,
) -> Result<Option<&'static str>, ComputeFileMimeError> {
    match file {
        TempFile::File { path, .. } => {
            let path = match path {
                Either::Left(path) => path.to_path_buf(),
                Either::Right(path) => path.to_owned(),
            };

            tokio::task::spawn_blocking(move || {
                let mime = infer::get_from_path(&path)
                    .map_err(|err| ComputeFileMimeError::InferError(err))?;

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
        TempFile::Buffered { content } => match infer::get(&content) {
            Some(mime) => Ok(Some(mime.mime_type())),
            None => Ok(None),
        },
    }
}
