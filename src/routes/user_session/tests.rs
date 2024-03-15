use super::dto::CreatingUserSession;
use crate::{
    db::models::{User, UserSession},
    routes::user::dto::CreatingUser,
    services::{AuthService, UserService},
    test::{create_test_rocket_instance, helpers::create_initial_user},
};
use rocket::{
    http::{Accept, ContentType, Header, Status},
    local::asynchronous::Client,
};
use std::sync::Arc;

#[rocket::async_test]
async fn test_create_user_session() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let username = "user";
    let email = "user@example.com";
    let password = "user";

    let response = client
        .post("/users")
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(
            serde_json::to_string(&CreatingUser {
                username,
                email,
                password,
            })
            .unwrap(),
        )
        .dispatch()
        .await;

    let status = response.status();
    let created_user = response.into_json::<User>().await.unwrap();

    assert_eq!(status, Status::Created);
    assert_eq!(created_user.username, username);
    assert_eq!(created_user.email, email);

    let response = client
        .post("/user-sessions")
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(serde_json::to_string(&CreatingUserSession { email, password }).unwrap())
        .dispatch()
        .await;

    let status = response.status();
    let user_session = response.into_json::<UserSession>().await.unwrap();

    assert_eq!(status, Status::Created);
    assert_eq!(user_session.user_id, created_user.id);

    let raw_user = auth_service
        .get_user_from_session(&user_session.token)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_user, created_user);
}

#[rocket::async_test]
async fn test_remove_user_session() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let response = client
        .delete("/user-sessions")
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let user_session = response.into_json::<UserSession>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(user_session, initial_user_session);

    let raw_user = auth_service
        .get_user_from_session(&user_session.token)
        .await
        .unwrap();

    assert_eq!(raw_user, None);
}
