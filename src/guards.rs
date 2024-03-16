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

fn make_bad_request<T>(msg: impl Into<String>) -> Outcome<T, Error> {
    Outcome::Error((
        Status::BadRequest,
        Error::new_dynamic(Status::BadRequest, msg),
    ))
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct OffsetHeader {
    pub offset: Option<u64>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for OffsetHeader {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let offset = match request.headers().get_one("Offset") {
            Some(offset) => match offset.parse::<u64>() {
                Ok(offset) => Some(offset),
                Err(_) => {
                    return make_bad_request(format!(
                        "offset `{}` in header is invalid; it should be non-negative integer.",
                        offset
                    ));
                }
            },
            None => None,
        };

        Outcome::Success(Self { offset })
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct RangeHeader {
    pub range: Option<(i64, Option<i64>)>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RangeHeader {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let range = match request.headers().get_one("Range") {
            Some(range) => range,
            None => {
                return Outcome::Success(Self { range: None });
            }
        };

        let range = range.trim();

        if !range.starts_with("bytes=") {
            return make_bad_request("range header should start with `bytes=`.");
        }

        let range = range.strip_prefix("bytes").unwrap_or(range).trim();
        let range = range.strip_prefix("=").unwrap_or(range).trim();

        // ignore multiple ranges
        let range = match range.split_once(',') {
            Some((range, _)) => range,
            None => range,
        };

        let (start, end) = if range.starts_with('-') {
            // suffix pattern: -start
            (range, None)
        } else {
            match range.split_once('-') {
                Some((start, end)) => (start.trim(), Some(end.trim())),
                None => (range, None),
            }
        };

        let start = match start.parse::<i64>() {
            Ok(start) => start,
            Err(_) => {
                return make_bad_request(format!(
                    "start `{}` in range header is invalid; it should be integer.",
                    start
                ));
            }
        };
        let end = match end {
            Some(end) if !end.is_empty() => match end.parse::<i64>() {
                Ok(end) => Some(end),
                Err(_) => {
                    return make_bad_request(format!(
                        "end `{}` in range header is invalid; it should be integer.",
                        end
                    ));
                }
            },
            _ => None,
        };

        match end {
            Some(end) => {
                // pattern: start-end
                // start and end must be non-negative integers
                if start < 0 {
                    return make_bad_request(format!(
                        "start `{}` in range header is less than 0.",
                        start
                    ));
                }

                if end < 0 {
                    return make_bad_request(format!(
                        "end `{}` in range header is less than 0.",
                        end
                    ));
                }

                // start must be less than or equal to end
                if end < start {
                    return make_bad_request(format!(
                        "start `{}` in range header is greater than end `{}`.",
                        start, end
                    ));
                }
            }
            _ if range.ends_with("-") => {
                // pattern: start-
                // start must be non-negative integer
                if start < 0 {
                    return make_bad_request(format!(
                        "start `{}` in range header is less than 0.",
                        start
                    ));
                }
            }
            _ => {
                // pattern: start
                // start can be negative in this case
            }
        }

        Outcome::Success(Self {
            range: Some((start, end)),
        })
    }
}
