use crate::{
    db::models::File,
    services::{AuthService, FileService, ReadRange, StagingFileService, UserService},
    test::{
        create_test_rocket_instance,
        helpers::{create_filled_staging_file, create_initial_user},
    },
};
use rocket::{
    http::{Accept, ContentType, Header, Status},
    local::asynchronous::Client,
};
use std::sync::Arc;
use tokio::io::AsyncReadExt;

#[rocket::async_test]
async fn test_create_file() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let file_service = client.rocket().state::<Arc<FileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let name = "file";
    let mime = "video/mp4";
    let file_content = "file content";

    let filled_staging_file = create_filled_staging_file(
        &client,
        staging_file_service,
        &initial_user_session,
        name,
        Some(mime),
        file_content,
    )
    .await;

    let response = client
        .post(format!("/files/{}", filled_staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let created_file = response.into_json::<File>().await.unwrap();

    assert_eq!(status, Status::Created);
    assert_eq!(created_file.name, name);
    assert_eq!(created_file.mime.as_str(), mime);
    assert_eq!(created_file.size, file_content.len() as i64);
    assert_eq!(created_file.hash, 0xD0D30AAE);

    let raw_created_file = file_service
        .get_file_by_id(created_file.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_created_file, created_file);
}

#[rocket::async_test]
async fn test_remove_file() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let file_service = client.rocket().state::<Arc<FileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let filled_staging_file = create_filled_staging_file(
        &client,
        staging_file_service,
        &initial_user_session,
        "file",
        Some("video/mp4"),
        "file content",
    )
    .await;

    let response = client
        .post(format!("/files/{}", filled_staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let file = response.into_json::<File>().await.unwrap();

    let response = client
        .delete(format!("/files/{}", file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let removed_file = response.into_json::<File>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(removed_file, file);

    let raw_removed_file = file_service.get_file_by_id(removed_file.id).await.unwrap();

    assert_eq!(raw_removed_file, None);

    let raw_removed_file_data = file_service
        .get_file_data_by_id(removed_file.id, ReadRange::Full)
        .await
        .unwrap();

    assert!(raw_removed_file_data.is_none());
}

#[rocket::async_test]
async fn test_get_file() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let file_service = client.rocket().state::<Arc<FileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let filled_staging_file = create_filled_staging_file(
        &client,
        staging_file_service,
        &initial_user_session,
        "file",
        Some("video/mp4"),
        "file content",
    )
    .await;

    let response = client
        .post(format!("/files/{}", filled_staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let file = response.into_json::<File>().await.unwrap();

    let response = client
        .get(format!("/files/{}", file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let retrieved_file = response.into_json::<File>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(retrieved_file, file);

    let raw_retrieved_file = file_service
        .get_file_by_id(retrieved_file.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_retrieved_file, retrieved_file);
}

#[rocket::async_test]
async fn test_get_file_data_range_full() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let file_service = client.rocket().state::<Arc<FileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let file_content = "file content";

    let filled_staging_file = create_filled_staging_file(
        &client,
        staging_file_service,
        &initial_user_session,
        "file",
        Some("video/mp4"),
        file_content,
    )
    .await;

    let response = client
        .post(format!("/files/{}", filled_staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let file = response.into_json::<File>().await.unwrap();

    let response = client
        .get(format!("/files/{}/data", file.id))
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let content_type = response.content_type().unwrap();
    let retrieved_file_data = response.into_string().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert!(content_type.is_mp4());
    assert_eq!(retrieved_file_data, file_content);

    let mut raw_retrieved_file_data = file_service
        .get_file_data_by_id(file.id, ReadRange::Full)
        .await
        .unwrap()
        .unwrap();
    let raw_retrieved_file_data = {
        let mut buffer = String::with_capacity(file_content.len());
        raw_retrieved_file_data
            .read_to_string(&mut buffer)
            .await
            .unwrap();
        buffer
    };

    assert_eq!(raw_retrieved_file_data, file_content);
}

#[rocket::async_test]
async fn test_get_file_data_range_start() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let file_service = client.rocket().state::<Arc<FileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let file_content = "file content";
    let range_start = 5;

    let filled_staging_file = create_filled_staging_file(
        &client,
        staging_file_service,
        &initial_user_session,
        "file",
        Some("video/mp4"),
        file_content,
    )
    .await;

    let file_content = &file_content[range_start..];

    let response = client
        .post(format!("/files/{}", filled_staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let file = response.into_json::<File>().await.unwrap();

    let response = client
        .get(format!("/files/{}/data", file.id))
        .header(Header::new("Range", format!("bytes={}", range_start)))
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let content_type = response.content_type().unwrap();
    let retrieved_file_data = response.into_string().await.unwrap();

    assert_eq!(status, Status::PartialContent);
    assert!(content_type.is_mp4());
    assert_eq!(retrieved_file_data, file_content);

    let mut raw_retrieved_file_data = file_service
        .get_file_data_by_id(file.id, ReadRange::Start(range_start as u64))
        .await
        .unwrap()
        .unwrap();
    let raw_retrieved_file_data = {
        let mut buffer = String::with_capacity(file_content.len());
        raw_retrieved_file_data
            .read_to_string(&mut buffer)
            .await
            .unwrap();
        buffer
    };

    assert_eq!(raw_retrieved_file_data, file_content);
}

#[rocket::async_test]
async fn test_get_file_data_range_min() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let file_service = client.rocket().state::<Arc<FileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let file_content = "file content";
    let range_start = 0;

    let filled_staging_file = create_filled_staging_file(
        &client,
        staging_file_service,
        &initial_user_session,
        "file",
        Some("video/mp4"),
        file_content,
    )
    .await;

    let file_content = &file_content[range_start..];

    let response = client
        .post(format!("/files/{}", filled_staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let file = response.into_json::<File>().await.unwrap();

    let response = client
        .get(format!("/files/{}/data", file.id))
        .header(Header::new("Range", format!("bytes={}", range_start)))
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let content_type = response.content_type().unwrap();
    let retrieved_file_data = response.into_string().await.unwrap();

    assert_eq!(status, Status::PartialContent);
    assert!(content_type.is_mp4());
    assert_eq!(retrieved_file_data, file_content);

    let mut raw_retrieved_file_data = file_service
        .get_file_data_by_id(file.id, ReadRange::Start(range_start as u64))
        .await
        .unwrap()
        .unwrap();
    let raw_retrieved_file_data = {
        let mut buffer = String::with_capacity(file_content.len());
        raw_retrieved_file_data
            .read_to_string(&mut buffer)
            .await
            .unwrap();
        buffer
    };

    assert_eq!(raw_retrieved_file_data, file_content);
}

#[rocket::async_test]
async fn test_get_file_data_range_max() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let file_service = client.rocket().state::<Arc<FileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let file_content = "file content";
    let range_start = file_content.len() - 1;

    let filled_staging_file = create_filled_staging_file(
        &client,
        staging_file_service,
        &initial_user_session,
        "file",
        Some("video/mp4"),
        file_content,
    )
    .await;

    let file_content = &file_content[range_start..];

    let response = client
        .post(format!("/files/{}", filled_staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let file = response.into_json::<File>().await.unwrap();

    let response = client
        .get(format!("/files/{}/data", file.id))
        .header(Header::new("Range", format!("bytes={}", range_start)))
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let content_type = response.content_type().unwrap();
    let retrieved_file_data = response.into_string().await.unwrap();

    assert_eq!(status, Status::PartialContent);
    assert!(content_type.is_mp4());
    assert_eq!(retrieved_file_data, file_content);

    let mut raw_retrieved_file_data = file_service
        .get_file_data_by_id(file.id, ReadRange::Start(range_start as u64))
        .await
        .unwrap()
        .unwrap();
    let raw_retrieved_file_data = {
        let mut buffer = String::with_capacity(file_content.len());
        raw_retrieved_file_data
            .read_to_string(&mut buffer)
            .await
            .unwrap();
        buffer
    };

    assert_eq!(raw_retrieved_file_data, file_content);
}

#[rocket::async_test]
async fn test_get_file_data_range_end() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let file_service = client.rocket().state::<Arc<FileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let file_content = "file content";
    let range_start = 5;
    let range_end = 10;

    let filled_staging_file = create_filled_staging_file(
        &client,
        staging_file_service,
        &initial_user_session,
        "file",
        Some("video/mp4"),
        file_content,
    )
    .await;

    let file_content = &file_content[range_start..=range_end];

    let response = client
        .post(format!("/files/{}", filled_staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let file = response.into_json::<File>().await.unwrap();

    let response = client
        .get(format!("/files/{}/data", file.id))
        .header(Header::new(
            "Range",
            format!("bytes={}-{}", range_start, range_end),
        ))
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let content_type = response.content_type().unwrap();
    let retrieved_file_data = response.into_string().await.unwrap();

    assert_eq!(status, Status::PartialContent);
    assert!(content_type.is_mp4());
    assert_eq!(retrieved_file_data, file_content);

    let mut raw_retrieved_file_data = file_service
        .get_file_data_by_id(
            file.id,
            ReadRange::Range(range_start as u64, range_end as u64),
        )
        .await
        .unwrap()
        .unwrap();
    let raw_retrieved_file_data = {
        let mut buffer = String::with_capacity(file_content.len());
        raw_retrieved_file_data
            .read_to_string(&mut buffer)
            .await
            .unwrap();
        buffer
    };

    assert_eq!(raw_retrieved_file_data, file_content);
}

#[rocket::async_test]
async fn test_get_file_data_range_end_min() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let file_service = client.rocket().state::<Arc<FileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let file_content = "file content";
    let range_start = 5;
    let range_end = range_start;

    let filled_staging_file = create_filled_staging_file(
        &client,
        staging_file_service,
        &initial_user_session,
        "file",
        Some("video/mp4"),
        file_content,
    )
    .await;

    let file_content = &file_content[range_start..=range_end];

    let response = client
        .post(format!("/files/{}", filled_staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let file = response.into_json::<File>().await.unwrap();

    let response = client
        .get(format!("/files/{}/data", file.id))
        .header(Header::new(
            "Range",
            format!("bytes={}-{}", range_start, range_end),
        ))
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let content_type = response.content_type().unwrap();
    let retrieved_file_data = response.into_string().await.unwrap();

    assert_eq!(status, Status::PartialContent);
    assert!(content_type.is_mp4());
    assert_eq!(retrieved_file_data, file_content);

    let mut raw_retrieved_file_data = file_service
        .get_file_data_by_id(
            file.id,
            ReadRange::Range(range_start as u64, range_end as u64),
        )
        .await
        .unwrap()
        .unwrap();
    let raw_retrieved_file_data = {
        let mut buffer = String::with_capacity(file_content.len());
        raw_retrieved_file_data
            .read_to_string(&mut buffer)
            .await
            .unwrap();
        buffer
    };

    assert_eq!(raw_retrieved_file_data, file_content);
}

#[rocket::async_test]
async fn test_get_file_data_range_end_max() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let file_service = client.rocket().state::<Arc<FileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let file_content = "file content";
    let range_start = 5;
    let range_end = file_content.len() - 1;

    let filled_staging_file = create_filled_staging_file(
        &client,
        staging_file_service,
        &initial_user_session,
        "file",
        Some("video/mp4"),
        file_content,
    )
    .await;

    let file_content = &file_content[range_start..=range_end];

    let response = client
        .post(format!("/files/{}", filled_staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let file = response.into_json::<File>().await.unwrap();

    let response = client
        .get(format!("/files/{}/data", file.id))
        .header(Header::new(
            "Range",
            format!("bytes={}-{}", range_start, range_end),
        ))
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let content_type = response.content_type().unwrap();
    let retrieved_file_data = response.into_string().await.unwrap();

    assert_eq!(status, Status::PartialContent);
    assert!(content_type.is_mp4());
    assert_eq!(retrieved_file_data, file_content);

    let mut raw_retrieved_file_data = file_service
        .get_file_data_by_id(
            file.id,
            ReadRange::Range(range_start as u64, range_end as u64),
        )
        .await
        .unwrap()
        .unwrap();
    let raw_retrieved_file_data = {
        let mut buffer = String::with_capacity(file_content.len());
        raw_retrieved_file_data
            .read_to_string(&mut buffer)
            .await
            .unwrap();
        buffer
    };

    assert_eq!(raw_retrieved_file_data, file_content);
}

#[rocket::async_test]
async fn test_get_file_data_range_suffix() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let file_service = client.rocket().state::<Arc<FileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let file_content = "file content";
    let range_suffix = 7;

    let filled_staging_file = create_filled_staging_file(
        &client,
        staging_file_service,
        &initial_user_session,
        "file",
        Some("video/mp4"),
        file_content,
    )
    .await;

    let file_content = &file_content[file_content.len() - range_suffix..];

    let response = client
        .post(format!("/files/{}", filled_staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let file = response.into_json::<File>().await.unwrap();

    let response = client
        .get(format!("/files/{}/data", file.id))
        .header(Header::new("Range", format!("bytes=-{}", range_suffix)))
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let content_type = response.content_type().unwrap();
    let retrieved_file_data = response.into_string().await.unwrap();

    assert_eq!(status, Status::PartialContent);
    assert!(content_type.is_mp4());
    assert_eq!(retrieved_file_data, file_content);

    let mut raw_retrieved_file_data = file_service
        .get_file_data_by_id(file.id, ReadRange::Suffix(range_suffix as u32))
        .await
        .unwrap()
        .unwrap();
    let raw_retrieved_file_data = {
        let mut buffer = String::with_capacity(file_content.len());
        raw_retrieved_file_data
            .read_to_string(&mut buffer)
            .await
            .unwrap();
        buffer
    };

    assert_eq!(raw_retrieved_file_data, file_content);
}

#[rocket::async_test]
async fn test_get_file_data_range_suffix_overflow() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let staging_file_service = client.rocket().state::<Arc<StagingFileService>>().unwrap();
    let file_service = client.rocket().state::<Arc<FileService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let file_content = "file content";
    let range_suffix = 9999;

    let filled_staging_file = create_filled_staging_file(
        &client,
        staging_file_service,
        &initial_user_session,
        "file",
        Some("video/mp4"),
        file_content,
    )
    .await;

    let response = client
        .post(format!("/files/{}", filled_staging_file.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let file = response.into_json::<File>().await.unwrap();

    let response = client
        .get(format!("/files/{}/data", file.id))
        .header(Header::new("Range", format!("bytes=-{}", range_suffix)))
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let content_type = response.content_type().unwrap();
    let retrieved_file_data = response.into_string().await.unwrap();

    assert_eq!(status, Status::PartialContent);
    assert!(content_type.is_mp4());
    assert_eq!(retrieved_file_data, file_content);

    let mut raw_retrieved_file_data = file_service
        .get_file_data_by_id(file.id, ReadRange::Suffix(range_suffix as u32))
        .await
        .unwrap()
        .unwrap();
    let raw_retrieved_file_data = {
        let mut buffer = String::with_capacity(file_content.len());
        raw_retrieved_file_data
            .read_to_string(&mut buffer)
            .await
            .unwrap();
        buffer
    };

    assert_eq!(raw_retrieved_file_data, file_content);
}
