use super::dto::{CreatingUser, SettingUserPassword, SettingUserUsername, UserList};
use crate::{
    db::models::User,
    dto::StaticError,
    guards::UserSession,
    services::{UserService, UserServiceError},
};
use rocket::{
    delete, get, http::Status, post, put, routes, serde::json::Json, Build, Responder, Rocket,
    State,
};
use std::sync::Arc;
use thiserror::Error;

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount(
        "/users",
        routes![
            create_user,
            remove_user,
            get_users,
            get_user,
            set_user_username,
            set_user_password
        ],
    )
}

#[derive(Responder, Error, Debug)]
#[response(content_type = "json")]
enum Error {
    #[response(status = 404)]
    #[error("not found")]
    NotFoundError(StaticError),
    #[response(status = 500)]
    #[error("internal server error")]
    UserServiceError(StaticError),
}

impl Error {
    pub fn not_found_error() -> Self {
        Error::NotFoundError(StaticError::not_found())
    }
}

impl From<UserServiceError> for Error {
    fn from(_error: UserServiceError) -> Self {
        Error::UserServiceError(StaticError::internal_server_error())
    }
}

#[post("/", data = "<user>")]
async fn create_user(
    _user_session: UserSession,
    user_service: &State<Arc<UserService>>,
    user: Json<CreatingUser>,
) -> Result<(Status, Json<User>), Error> {
    let user = user_service
        .create_user(&user.username, &user.email, &user.password)
        .await?;

    Ok((Status::Created, Json(user)))
}

#[delete("/<user_id>")]
async fn remove_user(
    _user_session: UserSession,
    user_service: &State<Arc<UserService>>,
    user_id: i32,
) -> Result<(Status, Json<User>), Error> {
    let user = user_service.remove_user_by_id(user_id).await?;
    let user = match user {
        Some(user) => user,
        None => {
            return Err(Error::not_found_error());
        }
    };

    Ok((Status::Ok, Json(user)))
}

#[get("/?<last_user_id>&<limit>")]
async fn get_users(
    _user_session: UserSession,
    user_service: &State<Arc<UserService>>,
    last_user_id: Option<i32>,
    limit: Option<u32>,
) -> Result<(Status, Json<UserList>), Error> {
    let limit = limit.unwrap_or_else(|| 25);
    let limit = u32::max(1, limit);
    let limit = u32::min(limit, 100);
    let users = user_service.get_users(last_user_id, limit).await?;

    Ok((
        Status::Ok,
        Json(UserList {
            users,
            last_user_id,
            limit,
        }),
    ))
}

#[get("/<user_id>")]
async fn get_user(
    _user_session: UserSession,
    user_service: &State<Arc<UserService>>,
    user_id: i32,
) -> Result<(Status, Json<User>), Error> {
    let user = user_service.get_user_by_id(user_id).await?;
    let user = match user {
        Some(user) => user,
        None => {
            return Err(Error::not_found_error());
        }
    };

    Ok((Status::Ok, Json(user)))
}

#[put("/<user_id>/username", data = "<username>")]
async fn set_user_username(
    _user_session: UserSession,
    user_service: &State<Arc<UserService>>,
    user_id: i32,
    username: Json<SettingUserUsername>,
) -> Result<(Status, Json<User>), Error> {
    let user = user_service
        .set_user_username_by_id(user_id, &username.username)
        .await?;
    let user = match user {
        Some(user) => user,
        None => {
            return Err(Error::not_found_error());
        }
    };

    Ok((Status::Ok, Json(user)))
}

#[put("/<user_id>/password", data = "<password>")]
async fn set_user_password(
    _user_session: UserSession,
    user_service: &State<Arc<UserService>>,
    user_id: i32,
    password: Json<SettingUserPassword>,
) -> Result<(Status, Json<User>), Error> {
    let user = user_service
        .set_user_password_by_id(user_id, &password.password)
        .await?;
    let user = match user {
        Some(user) => user,
        None => {
            return Err(Error::not_found_error());
        }
    };

    Ok((Status::Ok, Json(user)))
}
