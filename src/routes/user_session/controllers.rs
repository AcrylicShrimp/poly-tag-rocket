use super::dto::CreatingUserSession;
use crate::{
    db::models::UserSession, dto::JsonRes, guards::AuthUserSession, services::AuthService,
};
use rocket::{delete, http::Status, post, routes, serde::json::Json, Build, Rocket, State};
use std::sync::Arc;

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount(
        "/user-sessions",
        routes![create_user_session, remove_user_session],
    )
}

#[post("/", data = "<body>")]
async fn create_user_session(
    auth_service: &State<Arc<AuthService>>,
    body: Json<CreatingUserSession<'_>>,
) -> JsonRes<UserSession> {
    let user_id = auth_service
        .authenticate_user(body.email, body.password)
        .await;

    let user_id = match user_id {
        Ok(Some(user_id)) => user_id,
        Ok(None) => {
            return Err(Status::Unauthorized.into());
        }
        Err(err) => {
            let body = body.into_inner();
            log::error!(target: "routes::user_session::controllers", controller = "create_user_session", service = "AuthService", body:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    let user_session = auth_service.create_user_session(user_id).await;

    let user_session = match user_session {
        Ok(user_session) => user_session,
        Err(err) => {
            let body = body.into_inner();
            log::error!(target: "routes::user_session::controllers", controller = "create_user_session", service = "AuthService", body:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Created, Json(user_session)))
}

#[delete("/")]
async fn remove_user_session(
    sess: AuthUserSession<'_>,
    auth_service: &State<Arc<AuthService>>,
) -> JsonRes<UserSession> {
    let user_session = auth_service
        .remove_user_session(sess.user.id, sess.token)
        .await;

    let user_session = match user_session {
        Ok(Some(user_session)) => user_session,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            log::error!(target: "routes::user_session::controllers", controller = "remove_user_session", service = "AuthService", sess:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Ok, Json(user_session)))
}
