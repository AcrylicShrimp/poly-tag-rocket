use crate::{db::models::User, dto::Error, services::AuthService};
use rocket::{
    http::Status,
    request::{FromRequest, Outcome, Request},
    State,
};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize, Debug, Clone, PartialEq)]
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
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let authorization = match request.headers().get_one("Authorization") {
            Some(token) => token,
            None => return Outcome::Error((Status::Unauthorized, Status::Unauthorized.into())),
        };
        let token = match parse_authorization_header(authorization) {
            Some(token) => token,
            None => return Outcome::Error((Status::Unauthorized, Status::Unauthorized.into())),
        };

        let auth_service = match request.guard::<&State<Arc<AuthService>>>().await {
            Outcome::Success(auth_service) => auth_service,
            Outcome::Error(err) => {
                log::error!(target: "guards::AuthUserSession", guard = "AuthUserSession", err:serde; "Failed to get AuthService from request guard.");
                return Outcome::Error((
                    Status::InternalServerError,
                    Status::InternalServerError.into(),
                ));
            }
            Outcome::Forward(status) => {
                return Outcome::Forward(status);
            }
        };

        let user = match auth_service.get_user_from_session(token).await {
            Ok(Some(user)) => user,
            Ok(None) => return Outcome::Error((Status::Unauthorized, Status::Unauthorized.into())),
            Err(err) => {
                log::error!(target: "guards::AuthUserSession", guard = "AuthUserSession", service = "AuthService", err:err; "Failed to get user from session.");
                return Outcome::Error((
                    Status::InternalServerError,
                    Status::InternalServerError.into(),
                ));
            }
        };

        Outcome::Success(AuthUserSession { user, token })
    }
}
