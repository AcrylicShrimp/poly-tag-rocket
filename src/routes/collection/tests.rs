use super::dto::{CollectionList, CreatingCollection};
use crate::{
    db::models::{Collection, User, UserSession},
    services::{AuthService, CollectionService, UserService},
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
async fn test_create_collection() {
    let (rocket, _database_dropper) = create_test_rocket_instance().await;
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
    let (rocket, _database_dropper) = create_test_rocket_instance().await;
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
    let (rocket, _database_dropper) = create_test_rocket_instance().await;
    let client = Client::tracked(rocket).await.unwrap();
    let auth_service = client.rocket().state::<Arc<AuthService>>().unwrap();
    let collection_service = client.rocket().state::<Arc<CollectionService>>().unwrap();
    let user_service = client.rocket().state::<Arc<UserService>>().unwrap();

    let (_initial_user, initial_user_session) =
        create_initial_user(auth_service, user_service).await;

    let collection = vec![
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
        .get(format!("/collections?limit={}", collection.len()))
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
    assert_eq!(retrieved_collections.collections.len(), collection.len());
    assert_eq!(retrieved_collections.last_collection_id, None);
    assert_eq!(retrieved_collections.limit, collection.len() as u32);
    assert_eq!(retrieved_collections.collections, collection);

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
async fn test_get_collection() {
    let (rocket, _database_dropper) = create_test_rocket_instance().await;
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
    let (rocket, _database_dropper) = create_test_rocket_instance().await;
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
            serde_json::to_string(&CreatingCollection {
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
    assert_eq!(
        updated_collection
            .description
            .as_ref()
            .map(|description| description.as_str()),
        new_description
    );

    let raw_updated_collection = collection_service
        .get_collection_by_id(updated_collection.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(raw_updated_collection, updated_collection);
}
