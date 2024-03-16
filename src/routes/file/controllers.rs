use crate::{
    db::models::File,
    dto::{Error, JsonRes},
    guards::AuthUserSession,
    services::{FileService, FileServiceError},
};
use rocket::{
    delete, get,
    http::{Status, StatusClass},
    post,
    response::stream::ReaderStream,
    routes,
    serde::json::Json,
    Build, Rocket, State,
};
use std::{pin::Pin, sync::Arc};
use tokio::io::AsyncRead;
use uuid::Uuid;

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount(
        "/files",
        routes![create_file, remove_file, get_file, get_file_data],
    )
}

fn map_file_service_err(err: &FileServiceError) -> Error {
    match err {
        FileServiceError::FileNotYetFilled => {
            Error::new_dynamic(Status::UnprocessableEntity, "staging file not yet filled")
        }
        _ => Status::InternalServerError.into(),
    }
}

#[post("/<staging_file_id>")]
async fn create_file(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    file_service: &State<Arc<FileService>>,
    staging_file_id: Uuid,
) -> JsonRes<File> {
    let file = file_service
        .create_file_from_staging_file_id(staging_file_id)
        .await;

    let file = match file {
        Ok(Some(file)) => file,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            let error = map_file_service_err(&err);

            if error.status().class() == StatusClass::ServerError {
                log::error!(target: "routes::file::controllers", controller = "create_file", service = "FileService", staging_file_id:serde, err:err; "Error returned from service.");
            }

            return Err(error);
        }
    };

    Ok((Status::Created, Json(file)))
}

#[delete("/<file_id>")]
async fn remove_file(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    file_service: &State<Arc<FileService>>,
    file_id: Uuid,
) -> JsonRes<File> {
    let file = file_service.remove_file_by_id(file_id).await;

    let file = match file {
        Ok(Some(file)) => file,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            log::error!(target: "routes::file::controllers", controller = "remove_file", service = "FileService", file_id:serde, err:err; "Error returned from service.");
            return Err(map_file_service_err(&err));
        }
    };

    Ok((Status::Ok, Json(file)))
}

#[get("/<file_id>")]
async fn get_file(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    file_service: &State<Arc<FileService>>,
    file_id: Uuid,
) -> JsonRes<File> {
    let file = file_service.get_file_by_id(file_id).await;

    let file = match file {
        Ok(Some(file)) => file,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            log::error!(target: "routes::file::controllers", controller = "get_file", service = "FileService", file_id:serde, err:err; "Error returned from service.");
            return Err(map_file_service_err(&err));
        }
    };

    Ok((Status::Ok, Json(file)))
}

#[get("/<file_id>/data")]
async fn get_file_data(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    file_service: &State<Arc<FileService>>,
    file_id: Uuid,
) -> Result<(Status, ReaderStream![Pin<Box<dyn AsyncRead + Send>>]), Error> {
    let data = file_service.get_file_data_by_id(file_id).await;

    let data = match data {
        Ok(Some(data)) => data,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            log::error!(target: "routes::file::controllers", controller = "get_file_data", service = "FileService", file_id:serde, err:err; "Error returned from service.");
            return Err(map_file_service_err(&err));
        }
    };

    let stream = ReaderStream::one(data);
    Ok((Status::Ok, stream))
}
