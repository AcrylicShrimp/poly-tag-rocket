use super::dto::{CollectionList, CreatingCollection, UpdatingCollection};
use crate::{
    db::models::Collection,
    services::{AuthService, CollectionService, UserService},
    test::{create_test_rocket_instance, helpers::create_initial_user},
};
use rocket::{
    http::{Accept, ContentType, Header, Status},
    local::asynchronous::Client,
};
use std::sync::Arc;

#[rocket::async_test]
async fn test_create_collection() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let collection_service = client.rocket().state::<Arc<CollectionService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let name = "collection";
    let description = Some("collection description");

    let response = client
        .post("/collections")
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(serde_json::to_string(&CreatingCollection { name, description }).unwrap())
        .dispatch()
        .await;

    let status = response.status();
    let created_collection = response.into_json::<Collection>().await.unwrap();

    assert_eq!(status, Status::Created);
    assert_eq!(created_collection.name, name);
    assert_eq!(
        created_collection
            .description
            .as_ref()
            .map(|description| description.as_str()),
        description
    );

    let raw_created_collection = collection_service
        .get_collection_by_id(created_collection.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_created_collection, created_collection);
}

#[rocket::async_test]
async fn test_remove_collection() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let collection_service = client.rocket().state::<Arc<CollectionService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let collection = collection_service
        .create_collection("collection", Some("collection description"))
        .await
        .unwrap();

    let response = client
        .delete(format!("/collections/{}", collection.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let removed_collection = response.into_json::<Collection>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(removed_collection, collection);

    let raw_removed_collection = collection_service
        .get_collection_by_id(removed_collection.id)
        .await
        .unwrap();

    assert_eq!(raw_removed_collection, None);
}

#[rocket::async_test]
async fn test_get_collections() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let collection_service = client.rocket().state::<Arc<CollectionService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let collections = vec![
        collection_service
            .create_collection("collection0", Some("collection0 description"))
            .await
            .unwrap(),
        collection_service
            .create_collection("collection1", Some("collection1 description"))
            .await
            .unwrap(),
        collection_service
            .create_collection("collection2", Some("collection2 description"))
            .await
            .unwrap(),
    ];

    let response = client
        .get(format!("/collections?limit={}", collections.len()))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let retrieved_collections = response.into_json::<CollectionList>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(retrieved_collections.last_collection_id, None);
    assert_eq!(retrieved_collections.limit, collections.len() as u32);
    assert_eq!(retrieved_collections.collections, collections);

    let raw_retrieved_collections = collection_service
        .get_collections(
            retrieved_collections.last_collection_id,
            retrieved_collections.limit,
        )
        .await
        .unwrap();

    assert_eq!(raw_retrieved_collections, retrieved_collections.collections);
}

#[rocket::async_test]
async fn test_get_collections_paginations() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let collection_service = client.rocket().state::<Arc<CollectionService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let collections = vec![
        collection_service
            .create_collection("collection0", Some("collection0 description"))
            .await
            .unwrap(),
        collection_service
            .create_collection("collection1", Some("collection1 description"))
            .await
            .unwrap(),
        collection_service
            .create_collection("collection2", Some("collection2 description"))
            .await
            .unwrap(),
        collection_service
            .create_collection("collection3", Some("collection3 description"))
            .await
            .unwrap(),
        collection_service
            .create_collection("collection4", Some("collection4 description"))
            .await
            .unwrap(),
        collection_service
            .create_collection("collection5", Some("collection5 description"))
            .await
            .unwrap(),
    ];

    for index in 0..=collections.len() {
        let url = if index == 0 {
            format!("/collections?limit={}", collections.len())
        } else {
            format!(
                "/collections?last_collection_id={}&limit={}",
                collections[index - 1].id,
                collections.len()
            )
        };

        let response = client
            .get(url)
            .header(Accept::JSON)
            .header(ContentType::JSON)
            .header(Header::new(
                "Authorization",
                format!("Bearer {}", initial_user_session.token),
            ))
            .dispatch()
            .await;

        let status = response.status();
        let retrieved_collections = response.into_json::<CollectionList>().await.unwrap();

        assert_eq!(status, Status::Ok);
        assert_eq!(
            retrieved_collections.last_collection_id,
            if index == 0 {
                None
            } else {
                Some(collections[index - 1].id)
            }
        );
        assert_eq!(retrieved_collections.limit, collections.len() as u32);
        assert_eq!(retrieved_collections.collections, collections[index..]);

        let raw_retrieved_collections = collection_service
            .get_collections(
                retrieved_collections.last_collection_id,
                retrieved_collections.limit,
            )
            .await
            .unwrap();

        assert_eq!(raw_retrieved_collections, retrieved_collections.collections);
    }
}

#[rocket::async_test]
async fn test_get_collection() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let collection_service = client.rocket().state::<Arc<CollectionService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let collection = collection_service
        .create_collection("collection", Some("collection description"))
        .await
        .unwrap();

    let response = client
        .get(format!("/collections/{}", collection.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .dispatch()
        .await;

    let status = response.status();
    let retrieved_collection = response.into_json::<Collection>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(retrieved_collection, collection);

    let raw_retrieved_collection = collection_service
        .get_collection_by_id(retrieved_collection.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_retrieved_collection, retrieved_collection);
}

#[rocket::async_test]
async fn test_update_collection() {
    let (rocket, _database_dropper, _index_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let collection_service = client.rocket().state::<Arc<CollectionService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let collection = collection_service
        .create_collection("collection", Some("collection description"))
        .await
        .unwrap();

    let new_name = "new_collection";
    let new_description = Some("new collection description");

    let response = client
        .put(format!("/collections/{}", collection.id))
        .header(Accept::JSON)
        .header(ContentType::JSON)
        .header(Header::new(
            "Authorization",
            format!("Bearer {}", initial_user_session.token),
        ))
        .body(
            serde_json::to_string(&UpdatingCollection {
                name: new_name,
                description: new_description,
            })
            .unwrap(),
        )
        .dispatch()
        .await;

    let status = response.status();
    let updated_collection = response.into_json::<Collection>().await.unwrap();

    assert_eq!(status, Status::Ok);
    assert_eq!(updated_collection.name, new_name);
    assert_eq!(updated_collection.description.as_deref(), new_description);

    let raw_updated_collection = collection_service
        .get_collection_by_id(updated_collection.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_updated_collection, updated_collection);
}
