use super::dto::CreatingStagingFile;
use crate::{
    db::models::StagingFile,
    services::{AuthService, StagingFileService, UserService},
    test::{create_test_rocket_instance, helpers::create_initial_user},
};
use rocket::{
    http::{Accept, ContentType, Header, Status},
    local::asynchronous::Client,
};
use std::sync::Arc;

#[rocket::async_test]
async fn test_create_staging_file() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
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

#[rocket::async_test]
async fn test_remove_staging_file() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let staging_file = staging_file_service
        .create_staging_file("staging_file", Some("video/mp4"))
        .await
        .unwrap();

    let response = client
        .delete(format!("/staging-files/{}", staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let removed_staging_file = response.into_json::<StagingFile>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(removed_staging_file, staging_file);

    let raw_removed_staging_file = staging_file_service
        .get_staging_file_by_id(removed_staging_file.id)
        .await
        .unwrap();

    assert_eq!(raw_removed_staging_file, None);
}

#[rocket::async_test]
async fn test_get_staging_file() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let staging_file = staging_file_service
        .create_staging_file("staging_file", Some("video/mp4"))
        .await
        .unwrap();

    let response = client
        .get(format!("/staging-files/{}", staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let retrieved_staging_file = response.into_json::<StagingFile>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(retrieved_staging_file, staging_file);

    let raw_retrieved_staging_file = staging_file_service
        .get_staging_file_by_id(retrieved_staging_file.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_retrieved_staging_file, retrieved_staging_file);
}

#[rocket::async_test]
async fn test_fill_staging_file() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let staging_file = staging_file_service
        .create_staging_file("staging_file", Some("video/mp4"))
        .await
        .unwrap();

    let file_content = "file content";

    let response = client
        .put(format!("/staging-files/{}/data", staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::Binary)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(file_content)
        .dispatch()
        .await;

    let status = response.status();
    let filled_staging_file = response.into_json::<StagingFile>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(filled_staging_file.name, staging_file.name);
    assert_eq!(filled_staging_file.mime, staging_file.mime);
    assert_eq!(filled_staging_file.size, file_content.len() as i64);

    let raw_filled_staging_file = staging_file_service
        .get_staging_file_by_id(filled_staging_file.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_filled_staging_file, filled_staging_file);
}

#[rocket::async_test]
async fn test_fill_staging_file_with_offset() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let staging_file = staging_file_service
        .create_staging_file("staging_file", Some("video/mp4"))
        .await
        .unwrap();

    let file_content = "file content";

    let response = client
        .put(format!("/staging-files/{}/data", staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::Binary)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(file_content)
        .dispatch()
        .await;

    let status = response.status();
    let filled_staging_file = response.into_json::<StagingFile>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(filled_staging_file.name, staging_file.name);
    assert_eq!(filled_staging_file.mime, staging_file.mime);
    assert_eq!(filled_staging_file.size, file_content.len() as i64);

    let raw_filled_staging_file = staging_file_service
        .get_staging_file_by_id(filled_staging_file.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_filled_staging_file, filled_staging_file);

    let extra_file_content = "new file content";
    let offset = 6;
    let file_content = format!("{}{}", &file_content[..offset], extra_file_content);

    let response = client
        .put(format!("/staging-files/{}/data", staging_file.id))
        .header(Accept::JSON)
        .header(Header::new("Offset", offset.to_string()))
        .header(ContentType::Binary)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(extra_file_content)
        .dispatch()
        .await;

    let status = response.status();
    let filled_staging_file = response.into_json::<StagingFile>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(filled_staging_file.name, staging_file.name);
    assert_eq!(filled_staging_file.mime, staging_file.mime);
    assert_eq!(filled_staging_file.size, file_content.len() as i64);

    let raw_filled_staging_file = staging_file_service
        .get_staging_file_by_id(filled_staging_file.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_filled_staging_file, filled_staging_file);
}

#[rocket::async_test]
async fn test_fill_staging_file_with_offset_min() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let staging_file = staging_file_service
        .create_staging_file("staging_file", Some("video/mp4"))
        .await
        .unwrap();

    let file_content = "file content";

    let response = client
        .put(format!("/staging-files/{}/data", staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::Binary)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(file_content)
        .dispatch()
        .await;

    let status = response.status();
    let filled_staging_file = response.into_json::<StagingFile>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(filled_staging_file.name, staging_file.name);
    assert_eq!(filled_staging_file.mime, staging_file.mime);
    assert_eq!(filled_staging_file.size, file_content.len() as i64);

    let raw_filled_staging_file = staging_file_service
        .get_staging_file_by_id(filled_staging_file.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_filled_staging_file, filled_staging_file);

    let extra_file_content = "new file content";
    let offset = 0;
    let file_content = format!("{}{}", &file_content[..offset], extra_file_content);

    let response = client
        .put(format!("/staging-files/{}/data", staging_file.id))
        .header(Accept::JSON)
        .header(Header::new("Offset", offset.to_string()))
        .header(ContentType::Binary)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(extra_file_content)
        .dispatch()
        .await;

    let status = response.status();
    let filled_staging_file = response.into_json::<StagingFile>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(filled_staging_file.name, staging_file.name);
    assert_eq!(filled_staging_file.mime, staging_file.mime);
    assert_eq!(filled_staging_file.size, file_content.len() as i64);

    let raw_filled_staging_file = staging_file_service
        .get_staging_file_by_id(filled_staging_file.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_filled_staging_file, filled_staging_file);
}

#[rocket::async_test]
async fn test_fill_staging_file_with_offset_max() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let staging_file = staging_file_service
        .create_staging_file("staging_file", Some("video/mp4"))
        .await
        .unwrap();

    let file_content = "file content";

    let response = client
        .put(format!("/staging-files/{}/data", staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::Binary)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(file_content)
        .dispatch()
        .await;

    let status = response.status();
    let filled_staging_file = response.into_json::<StagingFile>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(filled_staging_file.name, staging_file.name);
    assert_eq!(filled_staging_file.mime, staging_file.mime);
    assert_eq!(filled_staging_file.size, file_content.len() as i64);

    let raw_filled_staging_file = staging_file_service
        .get_staging_file_by_id(filled_staging_file.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_filled_staging_file, filled_staging_file);

    let extra_file_content = "new file content";
    let offset = file_content.len();
    let file_content = format!("{}{}", &file_content[..offset], extra_file_content);

    let response = client
        .put(format!("/staging-files/{}/data", staging_file.id))
        .header(Accept::JSON)
        .header(Header::new("Offset", offset.to_string()))
        .header(ContentType::Binary)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(extra_file_content)
        .dispatch()
        .await;

    let status = response.status();
    let filled_staging_file = response.into_json::<StagingFile>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(filled_staging_file.name, staging_file.name);
    assert_eq!(filled_staging_file.mime, staging_file.mime);
    assert_eq!(filled_staging_file.size, file_content.len() as i64);

    let raw_filled_staging_file = staging_file_service
        .get_staging_file_by_id(filled_staging_file.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_filled_staging_file, filled_staging_file);
}
