use super::dto::CreatingStagingFile;
use crate::{
    db::models::{StagingFile, User, UserSession},
    services::{AuthService, StagingFileService, UserService},
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
async fn test_create_staging_file() {
    let (rocket, _database_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let name = "staging_file";
    let mime = Some("video/mp4");

    let response = client
        .post("/staging-files")
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(serde_json::to_string(&CreatingStagingFile { name, mime }).unwrap())
        .dispatch()
        .await;

    let status = response.status();
    let created_staging_file = response.into_json::<StagingFile>().await.unwrap();

    assert_eq!(status, Status::Created);
    assert_eq!(created_staging_file.name, name);
    assert_eq!(
        created_staging_file.mime.as_ref().map(|mime| mime.as_str()),
        mime
    );

    let raw_staging_file = staging_file_service
        .get_staging_file_by_id(created_staging_file.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_staging_file, created_staging_file);
}
