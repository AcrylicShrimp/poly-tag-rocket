use crate::{
    db::models::{User, UserSession},
    services::{AuthService, UserService},
    test::create_test_rocket_instance,
};
use rocket::{
    http::{Accept, ContentType, Header, Status},
    local::asynchronous::Client,
};
use serde_json::json;
use std::sync::Arc;

async fn create_initial_user(
    auth_service: &AuthService,
    user_service: &UserService,
) -> (User, UserSession) {
    let user = user_service
        .create_user(
            "initial_user",
            "initial_user@example.com",
            "initial_user_pw",
        )
        .await
        .unwrap();
    let user_session = auth_service.create_user_session(user.id).await.unwrap();

    (user, user_session)
}

#[rocket::async_test]
async fn test_create_user() {
    let (rocket, _database_dropper) = create_test_rocket_instance();
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
            json! ({
                "username": username,
                "email": email,
                "password": password,
            })
            .to_string(),
        )
        .dispatch()
        .await;

    let status = response.status();
    let user = response.into_json::<User>().await.unwrap();

    assert_eq!(status, Status::Created);
    assert_eq!(user.username, username);
    assert_eq!(user.email, email);

    let created_user = user_service.get_user_by_id(user.id).await.unwrap().unwrap();

    assert_eq!(user.id, created_user.id);
    assert_eq!(user.username, created_user.username);
    assert_eq!(user.email, created_user.email);
    assert_eq!(user.joined_at, created_user.joined_at);
}
