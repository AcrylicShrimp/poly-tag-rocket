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

async fn create_user(id: &str, user_service: &UserService) -> User {
    let user = user_service
        .create_user(
            &format!("{}_user", id),
            &format!("{}_user@example.com", id),
            &format!("{}_user_pw", id),
        )
        .await
        .unwrap();
    user
}

async fn create_initial_user(
    auth_service: &AuthService,
    user_service: &UserService,
) -> (User, UserSession) {
    let user = create_user("initial", user_service).await;
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
    let created_user = response.into_json::<User>().await.unwrap();

    assert_eq!(status, Status::Created);
    assert_eq!(created_user.username, username);
    assert_eq!(created_user.email, email);

    let created_user = user_service
        .get_user_by_id(created_user.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(created_user.id, created_user.id);
    assert_eq!(created_user.username, created_user.username);
    assert_eq!(created_user.email, created_user.email);
    assert_eq!(created_user.joined_at, created_user.joined_at);
}

#[rocket::async_test]
async fn remove_user() {
    let (rocket, _database_dropper) = create_test_rocket_instance();
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let user = create_user("user", user_service).await;

    let response = client
        .delete(format!("/users/{}", user.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let removed_user = response.into_json::<User>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(removed_user.id, user.id);
    assert_eq!(removed_user.username, user.username);
    assert_eq!(removed_user.email, user.email);

    let removed_user = user_service.get_user_by_id(removed_user.id).await.unwrap();

    assert_eq!(removed_user, None);
}
