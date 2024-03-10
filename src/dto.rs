use rocket::Responder;
use serde::{Deserialize, Serialize};

#[derive(Responder, Serialize, Deserialize, Debug, Clone)]
pub struct Error {
    pub error: String,
}

#[derive(Responder, Serialize, Deserialize, Debug, Clone)]
pub struct StaticError {
    pub error: &'static str,
}

impl StaticError {
    pub fn not_found() -> Self {
        StaticError { error: "not found" }
    }

    pub fn unauthorized() -> Self {
        StaticError {
            error: "unauthorized",
        }
    }

    pub fn internal_server_error() -> Self {
        StaticError {
            error: "internal server error",
        }
    }
}
