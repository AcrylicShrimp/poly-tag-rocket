use super::dto::CreatingStagingFile;
use crate::{
    config::AppConfig,
    db::models::StagingFile,
    dto::{Error, JsonRes},
    guards::{AuthUserSession, OffsetHeader},
    services::{StagingFileService, WriteError},
};
use rocket::{
    delete, get, http::Status, post, put, routes, serde::json::Json, Build, Data, Rocket, State,
};
use std::sync::Arc;
use uuid::Uuid;

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount(
        "/staging-files",
        routes![
            create_staging_file,
            remove_staging_file,
            get_staging_file,
            fill_staging_file
        ],
    )
}

#[post("/", data = "<body>")]
async fn create_staging_file(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    staging_file_service: &State<Arc<StagingFileService>>,
    body: Json<CreatingStagingFile<'_>>,
) -> JsonRes<StagingFile> {
    let staging_file = staging_file_service
        .create_staging_file(body.name, body.mime)
        .await;

    let staging_file = match staging_file {
        Ok(staging_file) => staging_file,
        Err(err) => {
            let body = body.into_inner();
            log::error!(target: "routes::staging_file::controllers", controller = "create_collection", service = "CollectionService", body:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Created, Json(staging_file)))
}

#[delete("/<staging_file_id>")]
async fn remove_staging_file(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    staging_file_service: &State<Arc<StagingFileService>>,
    staging_file_id: Uuid,
) -> JsonRes<StagingFile> {
    let staging_file = staging_file_service
        .remove_staging_file_by_id(staging_file_id, None, true)
        .await;

    let staging_file = match staging_file {
        Ok(Some(staging_file)) => staging_file,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            log::error!(target: "routes::staging_file::controllers", controller = "remove_staging_file", service = "StagingFileService", staging_file_id:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Ok, Json(staging_file)))
}

#[get("/<staging_file_id>")]
async fn get_staging_file(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    staging_file_service: &State<Arc<StagingFileService>>,
    staging_file_id: Uuid,
) -> JsonRes<StagingFile> {
    let staging_file = staging_file_service
        .get_staging_file_by_id(staging_file_id)
        .await;

    let staging_file = match staging_file {
        Ok(Some(staging_file)) => staging_file,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            log::error!(target: "routes::staging_file::controllers", controller = "get_staging_file", service = "StagingFileService", staging_file_id:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Ok, Json(staging_file)))
}

#[put("/<staging_file_id>", data = "<body>")]
async fn fill_staging_file(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    app_config: &State<AppConfig>,
    staging_file_service: &State<Arc<StagingFileService>>,
    staging_file_id: Uuid,
    offset_header: OffsetHeader,
    body: Data<'_>,
) -> JsonRes<StagingFile> {
    let stream = body.open(app_config.limits.file);
    let staging_file = staging_file_service
        .fill_staging_file_by_id(staging_file_id, offset_header.offset, stream)
        .await;

    let staging_file = match staging_file {
        Ok(Ok(Some(staging_file))) => staging_file,
        Ok(Ok(None)) => {
            return Err(Status::NotFound.into());
        }
        Ok(Err(err)) => match err {
            WriteError::OffsetExceedsFileSize { offset, file_size } => {
                return Err(Error::new_dynamic(
                    Status::UnprocessableEntity,
                    format!(
                        "the offset `{}` exceeds the file size `{}`",
                        offset, file_size
                    ),
                ));
            }
            WriteError::FileTooLarge {
                max_size,
                file_size,
            } => {
                return Err(Error::new_dynamic(
                    Status::UnprocessableEntity,
                    format!(
                        "the file size `{}` exceeds the maximum file size `{}`",
                        file_size, max_size
                    ),
                ));
            }
            WriteError::OffsetTooLarge { max_offset, offset } => {
                return Err(Error::new_dynamic(
                    Status::UnprocessableEntity,
                    format!(
                        "the offset `{}` exceeds the maximum offset `{}`",
                        offset, max_offset
                    ),
                ));
            }
            WriteError::Write {
                io_error,
                file_size,
            } => {
                log::error!(target: "routes::staging_file::controllers", controller = "fill_staging_file", service = "StagingFileService", staging_file_id:serde, io_error:err, file_size; "Error returned from service.");
                return Err(Status::InternalServerError.into());
            }
        },
        Err(err) => {
            log::error!(target: "routes::staging_file::controllers", controller = "fill_staging_file", service = "StagingFileService", staging_file_id:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Ok, Json(staging_file)))
}
