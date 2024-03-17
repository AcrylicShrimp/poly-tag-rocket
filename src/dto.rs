use rocket::{http::Status, serde::json::Json, Responder};
use serde::Serialize;

#[derive(Responder, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum ErrorBodyKind {
    Static(&'static str),
    Dynamic(String),
}

#[derive(Responder, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[response(content_type = "json")]
pub struct ErrorBody {
    pub error: ErrorBodyKind,
}

#[derive(Responder, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Error((Status, Json<ErrorBody>));

impl Error {
    pub fn new_static(status: Status, message: &'static str) -> Self {
        Error((
            status,
            Json(ErrorBody {
                error: ErrorBodyKind::Static(message),
            }),
        ))
    }

    pub fn new_dynamic(status: Status, message: impl Into<String>) -> Self {
        Error((
            status,
            Json(ErrorBody {
                error: ErrorBodyKind::Dynamic(message.into()),
            }),
        ))
    }

    pub fn status(&self) -> Status {
        self.0 .0
    }
}

impl From<Status> for Error {
    fn from(value: Status) -> Self {
        let message = match value.code {
            400 => "bad request",
            401 => "unauthorized",
            402 => "payment required",
            403 => "forbidden",
            404 => "not found",
            405 => "method not allowed",
            406 => "not acceptable",
            407 => "proxy authentication required",
            408 => "request timeout",
            409 => "conflict",
            410 => "gone",
            411 => "length required",
            412 => "precondition failed",
            413 => "payload too large",
            414 => "uri too long",
            415 => "unsupported media type",
            416 => "range not satisfiable",
            417 => "expectation failed",
            418 => "i'm a teapot",
            421 => "misdirected request",
            422 => "unprocessable entity",
            423 => "locked",
            424 => "failed dependency",
            426 => "upgrade required",
            428 => "precondition required",
            429 => "too many requests",
            431 => "request header fields too large",
            451 => "unavailable for legal reasons",
            500 => "internal server error",
            501 => "not implemented",
            502 => "bad gateway",
            503 => "service unavailable",
            504 => "gateway timeout",
            505 => "http version not supported",
            506 => "variant also negotiates",
            507 => "insufficient storage",
            508 => "loop detected",
            510 => "not extended",
            511 => "network authentication required",
            _ => "unknown",
        };

        Self::new_static(value, message)
    }
}

pub type JsonRes<T> = Result<(Status, Json<T>), Error>;
