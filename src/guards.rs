use crate::{db::models::User, dto::StaticError, services::AuthService};
use rocket::{
    http::Status,
    request::{FromRequest, Outcome, Request},
    Responder, State,
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Responder, Error, Debug)]
#[response(content_type = "json")]
pub enum GuardError {
    #[response(status = 401)]
    #[error("unauthorized")]
    Unauthorized(StaticError),
    #[response(status = 500)]
    #[error("internal server error")]
    InternalServerError(StaticError),
}

impl GuardError {
    pub fn unauthorized() -> Self {
        GuardError::Unauthorized(StaticError::unauthorized())
    }

    pub fn internal_server_error() -> Self {
        GuardError::InternalServerError(StaticError::internal_server_error())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthUserSession<'a> {
    pub user: User,
    pub token: &'a str,
}

fn parse_authorization_header(authorization: &str) -> Option<&str> {
    let segments = authorization.trim().splitn(2, ' ').collect::<Vec<&str>>();

    if segments.len() != 2 || !segments[0].eq_ignore_ascii_case("bearer") {
        return None;
    }

    Some(segments[1])
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthUserSession<'r> {
    type Error = GuardError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let authorization = match request.headers().get_one("Authorization") {
            Some(token) => token,
            None => return Outcome::Error((Status::Unauthorized, GuardError::unauthorized())),
        };
        let token = match parse_authorization_header(authorization) {
            Some(token) => token,
            None => return Outcome::Error((Status::Unauthorized, GuardError::unauthorized())),
        };

        let auth_service = match request.guard::<&State<Arc<AuthService>>>().await {
            Outcome::Success(auth_service) => auth_service,
            Outcome::Error(_) => {
                // TODO: log error
                return Outcome::Error((
                    Status::InternalServerError,
                    GuardError::internal_server_error(),
                ));
            }
            Outcome::Forward(status) => {
                return Outcome::Forward(status);
            }
        };

        let user = match auth_service.get_user_from_session(token).await {
            Ok(Some(user)) => user,
            Ok(None) => return Outcome::Error((Status::Unauthorized, GuardError::unauthorized())),
            Err(_) => {
                // TODO: log error
                return Outcome::Error((
                    Status::InternalServerError,
                    GuardError::internal_server_error(),
                ));
            }
        };

        Outcome::Success(AuthUserSession { user, token })
    }
}
