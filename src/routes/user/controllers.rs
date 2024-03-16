use super::dto::{CreatingUser, SettingUserPassword, SettingUserUsername, UserList};
use crate::{db::models::User, dto::JsonRes, guards::AuthUserSession, services::UserService};
use rocket::{
    delete, get, http::Status, post, put, routes, serde::json::Json, Build, Rocket, State,
};
use std::sync::Arc;

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

#[post("/", data = "<body>")]
async fn create_user(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    user_service: &State<Arc<UserService>>,
    body: Json<CreatingUser<'_>>,
) -> JsonRes<User> {
    let user = user_service
        .create_user(body.username, body.email, body.password)
        .await;

    let user = match user {
        Ok(user) => user,
        Err(err) => {
            let body = body.into_inner();
            log::error!(target: "routes::user::controllers", controller = "create_user", service = "UserService", body:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Created, Json(user)))
}

#[delete("/<user_id>")]
async fn remove_user(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    user_service: &State<Arc<UserService>>,
    user_id: i32,
) -> JsonRes<User> {
    let user = user_service.remove_user_by_id(user_id).await;

    let user = match user {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            log::error!(target: "routes::user::controllers", controller = "remove_user", service = "UserService", user_id:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Ok, Json(user)))
}

#[get("/?<last_user_id>&<limit>")]
async fn get_users(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    user_service: &State<Arc<UserService>>,
    last_user_id: Option<i32>,
    limit: Option<u32>,
) -> JsonRes<UserList> {
    let limit = limit.unwrap_or(25);
    let limit = u32::max(1, limit);
    let limit = u32::min(limit, 100);

    let users = user_service.get_users(last_user_id, limit).await;

    let users = match users {
        Ok(users) => users,
        Err(err) => {
            log::error!(target: "routes::user::controllers", controller = "get_users", service = "UserService", last_user_id:serde, limit, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

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
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    user_service: &State<Arc<UserService>>,
    user_id: i32,
) -> JsonRes<User> {
    let user = user_service.get_user_by_id(user_id).await;

    let user = match user {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            log::error!(target: "routes::user::controllers", controller = "get_user", service = "UserService", user_id:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Ok, Json(user)))
}

#[put("/<user_id>/username", data = "<body>")]
async fn set_user_username(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    user_service: &State<Arc<UserService>>,
    user_id: i32,
    body: Json<SettingUserUsername<'_>>,
) -> JsonRes<User> {
    let user = user_service
        .set_user_username_by_id(user_id, body.username)
        .await;

    let user = match user {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            let body = body.into_inner();
            log::error!(target: "routes::user::controllers", controller = "set_user_username", service = "UserService", user_id:serde, body:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Ok, Json(user)))
}

#[put("/<user_id>/password", data = "<body>")]
async fn set_user_password(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    user_service: &State<Arc<UserService>>,
    user_id: i32,
    body: Json<SettingUserPassword<'_>>,
) -> JsonRes<User> {
    let user = user_service
        .set_user_password_by_id(user_id, body.password)
        .await;

    let user = match user {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            let body = body.into_inner();
            log::error!(target: "routes::user::controllers", controller = "set_user_password", service = "UserService", user_id:serde, body:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Ok, Json(user)))
}
