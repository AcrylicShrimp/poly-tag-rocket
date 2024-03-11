use crate::dto::StaticError;
use rocket::{routes, Build, Responder, Rocket};
use thiserror::Error;

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount("/files", routes![])
}

#[derive(Responder, Error, Debug)]
#[response(content_type = "json")]
enum Error {
    #[response(status = 404)]
    #[error("not found")]
    NotFoundError(StaticError),
    #[response(status = 500)]
    #[error("internal server error")]
    FileServiceError(StaticError),
}

impl Error {
    pub fn not_found_error() -> Self {
        Error::NotFoundError(StaticError::not_found())
    }
}
