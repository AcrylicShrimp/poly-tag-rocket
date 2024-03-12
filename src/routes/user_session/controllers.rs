use super::dto::CreatingUserSession;
use crate::{
    db::models::UserSession,
    dto::StaticError,
    guards::AuthUserSession,
    services::{AuthService, AuthServiceError},
};
use rocket::{
    delete, http::Status, post, routes, serde::json::Json, Build, Responder, Rocket, State,
};
use std::sync::Arc;
use thiserror::Error;

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount(
        "/user-sessions",
        routes![create_user_session, remove_user_session],
    )
}

#[derive(Responder, Error, Debug)]
#[response(content_type = "json")]
enum Error {
    #[response(status = 401)]
    #[error("unauthorized")]
    UnauthorizedError(StaticError),
    #[response(status = 404)]
    #[error("not found")]
    NotFoundError(StaticError),
    #[response(status = 500)]
    #[error("internal server error")]
    InternalServerError(StaticError),
}

impl Error {
    pub fn unauthorized_error() -> Self {
        Error::UnauthorizedError(StaticError::unauthorized())
    }

    pub fn not_found_error() -> Self {
        Error::NotFoundError(StaticError::not_found())
    }
}

impl From<AuthServiceError> for Error {
    fn from(_error: AuthServiceError) -> Self {
        Error::InternalServerError(StaticError::internal_server_error())
    }
}

#[post("/", data = "<user_session>")]
async fn create_user_session(
    auth_service: &State<Arc<AuthService>>,
    user_session: Json<CreatingUserSession<'_>>,
) -> Result<(Status, Json<UserSession>), Error> {
    let user_id = auth_service
        .authenticate_user(user_session.email, user_session.password)
        .await?;
    let user_id = match user_id {
        Some(user_id) => user_id,
        None => {
            return Err(Error::unauthorized_error());
        }
    };
    let user_session = auth_service.create_user_session(user_id).await?;

    Ok((Status::Created, Json(user_session)))
}

#[delete("/")]
async fn remove_user_session(
    user_session: AuthUserSession<'_>,
    auth_service: &State<Arc<AuthService>>,
) -> Result<(Status, Json<UserSession>), Error> {
    let user_session = auth_service
        .remove_user_session(user_session.user.id, user_session.token)
        .await?;
    let user_session = match user_session {
        Some(user_session) => user_session,
        None => {
            return Err(Error::not_found_error());
        }
    };

    Ok((Status::Ok, Json(user_session)))
}
