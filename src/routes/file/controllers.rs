use super::dto::{FileData, FileList, FileSearchResult, SearchingFile};
use crate::{
    db::models::File,
    dto::{Error, JsonRes},
    guards::{AuthUserSession, RangeHeader},
    services::{FileService, FileServiceError, ReadError, ReadRange, SearchService},
};
use rocket::{
    delete, get,
    http::{Status, StatusClass},
    post, routes,
    serde::json::Json,
    Build, Rocket, State,
};
use std::sync::Arc;
use uuid::Uuid;

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount(
        "/files",
        routes![
            create_file,
            remove_file,
            search_files,
            get_files,
            get_file,
            get_file_data
        ],
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

#[post("/search", data = "<body>")]
async fn search_files(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    search_service: &State<Arc<SearchService>>,
    body: Json<SearchingFile<'_>>,
) -> JsonRes<FileSearchResult> {
    let files = search_service
        .search_files(
            body.query,
            body.filter_mime,
            body.filter_size,
            body.filter_hash,
            body.filter_uploaded_at,
        )
        .await;

    let files = match files {
        Ok(files) => files,
        Err(err) => {
            let body = body.into_inner();
            log::error!(target: "routes::file::controllers", controller = "search_files", service = "SearchService", body:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Ok, Json(FileSearchResult { files })))
}

#[get("/?<last_file_id>&<limit>")]
async fn get_files(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    file_service: &State<Arc<FileService>>,
    last_file_id: Option<Uuid>,
    limit: Option<u32>,
) -> JsonRes<FileList> {
    let limit = limit.unwrap_or(25);
    let limit = u32::max(1, limit);
    let limit = u32::min(limit, 100);
    let files = file_service.get_files(last_file_id, limit).await;

    let files = match files {
        Ok(files) => files,
        Err(err) => {
            log::error!(target: "routes::file::controllers", controller = "get_files", service = "FileService", last_file_id:serde, limit, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((
        Status::Ok,
        Json(FileList {
            files,
            last_file_id,
            limit,
        }),
    ))
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
    range_header: RangeHeader,
    file_id: Uuid,
) -> Result<FileData, Error> {
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

    let read_range = match range_header.range {
        None => ReadRange::Full,
        Some((start, None)) => {
            if start < 0 {
                ReadRange::Suffix((-start) as u32)
            } else {
                ReadRange::Start(start as u64)
            }
        }
        Some((start, Some(end))) => ReadRange::Range(start as u64, end as u64),
    };

    let data = file_service
        .get_file_data_by_id(file_id, read_range.clone())
        .await;
    let data = match data {
        Ok(Some(data)) => data,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => match err {
            ReadError::RangeStartExceedsFileSize { start, file_size } => {
                return Err(Error::new_dynamic(
                    Status::RangeNotSatisfiable,
                    format!(
                        "the start of the range {} (inclusive) exceeds the file size {}",
                        start, file_size
                    ),
                ));
            }
            ReadError::RangeEndExceedsFileSize { end, file_size } => {
                return Err(Error::new_dynamic(
                    Status::RangeNotSatisfiable,
                    format!(
                        "the end of the range {} (inclusive) exceeds the file size {}",
                        end, file_size
                    ),
                ));
            }
            ReadError::Read { io_error } => {
                log::error!(target: "routes::file::controllers", controller = "get_file_data", service = "FileService", file_id:serde, io_error:err; "Error returned from service.");
                return Err(Status::InternalServerError.into());
            }
        },
    };

    Ok(FileData {
        status: match read_range {
            ReadRange::Full => Status::Ok,
            _ => Status::PartialContent,
        },
        mime: file.mime,
        data,
    })
}
