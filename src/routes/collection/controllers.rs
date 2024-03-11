use super::dto::{CollectionList, CreatingCollection, UpdatingCollection};
use crate::{
    db::models::Collection,
    dto::StaticError,
    guards::AuthUserSession,
    services::{CollectionService, CollectionServiceError},
};
use rocket::{
    delete, get, http::Status, post, put, routes, serde::json::Json, Build, Responder, Rocket,
    State,
};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount(
        "/collections",
        routes![
            create_collection,
            remove_collection,
            get_collections,
            get_collection,
            update_collection
        ],
    )
}

#[derive(Responder, Error, Debug)]
#[response(content_type = "json")]
enum Error {
    #[response(status = 404)]
    #[error("not found")]
    NotFoundError(StaticError),
    #[response(status = 500)]
    #[error("internal server error")]
    CollectionServiceError(StaticError),
}

impl Error {
    pub fn not_found_error() -> Self {
        Error::NotFoundError(StaticError::not_found())
    }
}

impl From<CollectionServiceError> for Error {
    fn from(_error: CollectionServiceError) -> Self {
        Error::CollectionServiceError(StaticError::internal_server_error())
    }
}

#[post("/", data = "<collection>")]
async fn create_collection(
    #[allow(unused_variables)] user_session: AuthUserSession<'_>,
    collection_service: &State<Arc<CollectionService>>,
    collection: Json<CreatingCollection<'_>>,
) -> Result<(Status, Json<Collection>), Error> {
    let collection = collection_service
        .create_collection(collection.name, collection.description)
        .await?;

    Ok((Status::Created, Json(collection)))
}

#[delete("/<collection_id>")]
async fn remove_collection(
    #[allow(unused_variables)] user_session: AuthUserSession<'_>,
    collection_service: &State<Arc<CollectionService>>,
    collection_id: Uuid,
) -> Result<(Status, Json<Collection>), Error> {
    let collection = collection_service
        .remove_collection_by_id(collection_id)
        .await?;
    let collection = match collection {
        Some(collection) => collection,
        None => {
            return Err(Error::not_found_error());
        }
    };

    Ok((Status::Ok, Json(collection)))
}

#[get("/?<last_collection_id>&<limit>")]
async fn get_collections(
    #[allow(unused_variables)] user_session: AuthUserSession<'_>,
    collection_service: &State<Arc<CollectionService>>,
    last_collection_id: Option<Uuid>,
    limit: Option<u32>,
) -> Result<(Status, Json<CollectionList>), Error> {
    let limit = limit.unwrap_or_else(|| 25);
    let limit = u32::max(1, limit);
    let limit = u32::min(limit, 100);
    let collections = collection_service
        .get_collections(last_collection_id, limit)
        .await?;

    Ok((
        Status::Ok,
        Json(CollectionList {
            collections,
            last_collection_id,
            limit,
        }),
    ))
}

#[get("/<collection_id>")]
async fn get_collection(
    #[allow(unused_variables)] user_session: AuthUserSession<'_>,
    collection_service: &State<Arc<CollectionService>>,
    collection_id: Uuid,
) -> Result<(Status, Json<Collection>), Error> {
    let collection = collection_service
        .get_collection_by_id(collection_id)
        .await?;
    let collection = match collection {
        Some(collection) => collection,
        None => {
            return Err(Error::not_found_error());
        }
    };

    Ok((Status::Ok, Json(collection)))
}

#[put("/<collection_id>", data = "<collection>")]
async fn update_collection(
    #[allow(unused_variables)] user_session: AuthUserSession<'_>,
    collection_service: &State<Arc<CollectionService>>,
    collection_id: Uuid,
    collection: Json<UpdatingCollection<'_>>,
) -> Result<(Status, Json<Collection>), Error> {
    let collection = collection_service
        .update_collection_by_id(collection_id, collection.name, collection.description)
        .await?;
    let collection = match collection {
        Some(collection) => collection,
        None => {
            return Err(Error::not_found_error());
        }
    };

    Ok((Status::Ok, Json(collection)))
}
