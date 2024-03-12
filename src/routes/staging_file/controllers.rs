use super::dto::CreatingStagingFile;
use crate::{
    config::AppConfig,
    db::models::StagingFile,
    dto::StaticError,
    guards::AuthUserSession,
    services::{StagingFileService, StagingFileServiceError, WriteError},
};
use rocket::{
    http::Status, post, put, routes, serde::json::Json, Build, Data, Responder, Rocket, State,
};
use std::sync::Arc;
use uuid::Uuid;

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount(
        "/staging-files",
        routes![create_staging_file, fill_staging_file],
    )
}

#[derive(Responder)]
#[response(content_type = "json")]
enum Error {
    #[response(status = 404)]
    NotFoundError(StaticError),
    #[response(status = 500)]
    InternalServerError(StaticError),
    Error((Status, Json<crate::dto::Error>)),
}

impl Error {
    pub fn not_found_error() -> Self {
        Error::NotFoundError(StaticError::not_found())
    }

    pub fn internal_server_error() -> Self {
        Error::InternalServerError(StaticError::internal_server_error())
    }

    pub fn error(status: Status, error: String) -> Self {
        Error::Error((status, Json(crate::dto::Error { error })))
    }
}

impl From<StagingFileServiceError> for Error {
    fn from(_error: StagingFileServiceError) -> Self {
        Error::InternalServerError(StaticError::internal_server_error())
    }
}

#[post("/", data = "<data>")]
async fn create_staging_file(
    #[allow(unused_variables)] user_session: AuthUserSession<'_>,
    staging_file_service: &State<Arc<StagingFileService>>,
    data: Json<CreatingStagingFile<'_>>,
) -> Result<(Status, Json<StagingFile>), Error> {
    let staging_file = staging_file_service
        .create_staging_file(data.name, data.mime)
        .await?;

    Ok((Status::Created, Json(staging_file)))
}

#[put("/<staging_file_id>", data = "<data>")]
async fn fill_staging_file(
    #[allow(unused_variables)] user_session: AuthUserSession<'_>,
    app_config: &State<AppConfig>,
    staging_file_service: &State<Arc<StagingFileService>>,
    staging_file_id: Uuid,
    data: Data<'_>,
) -> Result<(Status, Json<StagingFile>), Error> {
    let stream = data.open(app_config.limits.file);
    let staging_file = staging_file_service
        .fill_staging_file_by_id(staging_file_id, None, stream)
        .await?;

    let staging_file = match staging_file {
        Ok(staging_file) => staging_file,
        Err(err) => match err {
            WriteError::OffsetExceedsFileSize { offset, file_size } => {
                return Err(Error::error(
                    Status::UnprocessableEntity,
                    format!(
                        "the offset `{}` exceeds the file size `{}`",
                        offset, file_size
                    ),
                ));
            }
            WriteError::WriteError { .. } => {
                return Err(Error::internal_server_error());
            }
            WriteError::FileTooLarge {
                max_size,
                file_size,
            } => {
                return Err(Error::error(
                    Status::UnprocessableEntity,
                    format!(
                        "the file size `{}` exceeds the maximum file size `{}`",
                        file_size, max_size
                    ),
                ));
            }
            WriteError::OffsetTooLarge { max_offset, offset } => {
                return Err(Error::error(
                    Status::UnprocessableEntity,
                    format!(
                        "the offset `{}` exceeds the maximum offset `{}`",
                        offset, max_offset
                    ),
                ));
            }
        },
    };
    let staging_file = match staging_file {
        Some(staging_file) => staging_file,
        None => return Err(Error::not_found_error()),
    };

    Ok((Status::Ok, Json(staging_file)))
}
