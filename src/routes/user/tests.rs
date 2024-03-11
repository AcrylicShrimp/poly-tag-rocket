use super::dto::{CreatingUser, SettingUserPassword, SettingUserUsername, UserList};
use crate::{
    db::models::{User, UserSession},
    services::{AuthService, UserService},
    test::create_test_rocket_instance,
};
use rocket::{
    http::{Accept, ContentType, Header, Status},
    local::asynchronous::Client,
};
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
    let (rocket, _database_dropper) = create_test_rocket_instance().await;
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

    let raw_created_user = user_service
        .get_user_by_id(created_user.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_created_user, created_user);
}

#[rocket::async_test]
async fn test_remove_user() {
    let (rocket, _database_dropper) = create_test_rocket_instance().await;
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
    assert_eq!(removed_user, user);

    let raw_removed_user = user_service.get_user_by_id(removed_user.id).await.unwrap();

    assert_eq!(raw_removed_user, None);
}

#[rocket::async_test]
async fn test_get_users() {
    let (rocket, _database_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let users = vec![
        create_user("user0", user_service).await,
        create_user("user1", user_service).await,
        create_user("user2", user_service).await,
    ];

    let response = client
        .get(format!(
            "/users?last_user_id={}&limit={}",
            initial_user.id,
            users.len()
        ))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let retrieved_users = response.into_json::<UserList>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(retrieved_users.users.len(), users.len());
    assert_eq!(retrieved_users.last_user_id, Some(initial_user.id));
    assert_eq!(retrieved_users.limit, users.len() as u32);
    assert_eq!(retrieved_users.users, users);

    let raw_retrieved_users = user_service
        .get_users(retrieved_users.last_user_id, retrieved_users.limit)
        .await
        .unwrap();

    assert_eq!(raw_retrieved_users, retrieved_users.users);
}

#[rocket::async_test]
async fn test_get_user() {
    let (rocket, _database_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let user = create_user("user", user_service).await;

    let response = client
        .get(format!("/users/{}", user.id,))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let retrieved_user = response.into_json::<User>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(retrieved_user, user);

    let raw_retrieved_user = user_service
        .get_user_by_id(retrieved_user.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_retrieved_user, retrieved_user);
}

#[rocket::async_test]
async fn test_set_user_username() {
    let (rocket, _database_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let user = create_user("user", user_service).await;
    let new_username = "new_username";

    let response = client
        .put(format!("/users/{}/username", user.id,))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(
            serde_json::to_string(&SettingUserUsername {
                username: new_username,
            })
            .unwrap(),
        )
        .dispatch()
        .await;

    let status = response.status();
    let updated_user = response.into_json::<User>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(updated_user.id, user.id);
    assert_eq!(updated_user.username, new_username);
    assert_eq!(updated_user.email, user.email);
    assert_eq!(updated_user.joined_at, user.joined_at);

    let raw_updated_user = user_service
        .get_user_by_id(updated_user.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_updated_user, updated_user);
}

#[rocket::async_test]
async fn test_set_user_password() {
    let (rocket, _database_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let user = create_user("user", user_service).await;
    let new_password = "new_password";

    let response = client
        .put(format!("/users/{}/password", user.id,))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(
            serde_json::to_string(&SettingUserPassword {
                password: new_password,
            })
            .unwrap(),
        )
        .dispatch()
        .await;

    let status = response.status();
    let updated_user = response.into_json::<User>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(updated_user, user);

    let raw_updated_user = user_service
        .get_user_by_id(updated_user.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_updated_user, updated_user);

    let authenticated_user_id = auth_service
        .authenticate_user(&user.email, new_password)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(authenticated_user_id, user.id);
}
