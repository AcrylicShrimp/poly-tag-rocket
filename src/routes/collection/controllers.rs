use super::dto::{
    AddingCollectionFile, CollectionList, CollectionSearchResult, CreatingCollection,
    SearchingCollection, UpdatingCollection,
};
use crate::{
    db::models::{Collection, CollectionFilePair},
    dto::{Error, JsonRes},
    guards::AuthUserSession,
    services::{
        AddFileToCollectionError, CollectionFilePairService, CollectionService,
        RemoveFileFromCollectionError, SearchService,
    },
};
use rocket::{
    delete, get, http::Status, post, put, routes, serde::json::Json, Build, Rocket, State,
};
use std::sync::Arc;
use uuid::Uuid;

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount(
        "/collections",
        routes![
            create_collection,
            remove_collection,
            search_collections,
            get_collections,
            get_collection,
            update_collection,
            add_file_to_collection,
            remove_file_from_collection,
        ],
    )
}

#[post("/", data = "<body>")]
async fn create_collection(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    collection_service: &State<Arc<CollectionService>>,
    body: Json<CreatingCollection<'_>>,
) -> JsonRes<Collection> {
    let collection = collection_service
        .create_collection(body.name, body.description)
        .await;

    let collection = match collection {
        Ok(collection) => collection,
        Err(err) => {
            let body = body.into_inner();
            log::error!(target: "routes::collection::controllers", controller = "create_collection", service = "CollectionService", body:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Created, Json(collection)))
}

#[delete("/<collection_id>")]
async fn remove_collection(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    collection_service: &State<Arc<CollectionService>>,
    collection_id: Uuid,
) -> JsonRes<Collection> {
    let collection = collection_service
        .remove_collection_by_id(collection_id)
        .await;

    let collection = match collection {
        Ok(Some(collection)) => collection,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            log::error!(target: "routes::collection::controllers", controller = "remove_collection", service = "CollectionService", collection_id:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Ok, Json(collection)))
}

#[post("/search", data = "<body>")]
async fn search_collections(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    search_service: &State<Arc<SearchService>>,
    body: Json<SearchingCollection<'_>>,
) -> JsonRes<CollectionSearchResult> {
    let collections = search_service.search_collections(body.query).await;

    let collections = match collections {
        Ok(collections) => collections,
        Err(err) => {
            let body = body.into_inner();
            log::error!(target: "routes::collection::controllers", controller = "search_collections", service = "SearchService", body:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Ok, Json(CollectionSearchResult { collections })))
}

#[get("/?<last_collection_id>&<limit>")]
async fn get_collections(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    collection_service: &State<Arc<CollectionService>>,
    last_collection_id: Option<Uuid>,
    limit: Option<u32>,
) -> JsonRes<CollectionList> {
    let limit = limit.unwrap_or(25);
    let limit = u32::max(1, limit);
    let limit = u32::min(limit, 100);
    let collections = collection_service
        .get_collections(last_collection_id, limit)
        .await;

    let collections = match collections {
        Ok(collections) => collections,
        Err(err) => {
            log::error!(target: "routes::collection::controllers", controller = "get_collections", service = "CollectionService", last_collection_id:serde, limit, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

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
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    collection_service: &State<Arc<CollectionService>>,
    collection_id: Uuid,
) -> JsonRes<Collection> {
    let collection = collection_service.get_collection_by_id(collection_id).await;

    let collection = match collection {
        Ok(Some(collection)) => collection,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            log::error!(target: "routes::collection::controllers", controller = "get_collection", service = "CollectionService", collection_id:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Ok, Json(collection)))
}

#[put("/<collection_id>", data = "<body>")]
async fn update_collection(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    collection_service: &State<Arc<CollectionService>>,
    collection_id: Uuid,
    body: Json<UpdatingCollection<'_>>,
) -> JsonRes<Collection> {
    let collection = collection_service
        .update_collection_by_id(collection_id, body.name, body.description)
        .await;

    let collection = match collection {
        Ok(Some(collection)) => collection,
        Ok(None) => {
            return Err(Status::NotFound.into());
        }
        Err(err) => {
            let body = body.into_inner();
            log::error!(target: "routes::collection::controllers", controller = "update_collection", service = "CollectionService", collection_id:serde, body:serde, err:err; "Error returned from service.");
            return Err(Status::InternalServerError.into());
        }
    };

    Ok((Status::Ok, Json(collection)))
}

/// TODO: add a test for this route
#[post("/<collection_id>/files", data = "<body>")]
async fn add_file_to_collection(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    collection_file_pair_service: &State<Arc<CollectionFilePairService>>,
    collection_id: Uuid,
    body: Json<AddingCollectionFile>,
) -> JsonRes<CollectionFilePair> {
    let pair = collection_file_pair_service
        .add_file_to_collection(collection_id, body.file_id)
        .await;

    let pair = match pair {
        Ok(pair) => pair,
        Err(err) => match err {
            AddFileToCollectionError::AlreadyExists { .. } => {
                return Err(Error::new_dynamic(Status::Conflict, err.to_string()));
            }
            AddFileToCollectionError::InvalidCollection { .. } => {
                return Err(Error::new_dynamic(Status::NotFound, err.to_string()));
            }
            AddFileToCollectionError::InvalidFile { .. } => {
                return Err(Error::new_dynamic(
                    Status::UnprocessableEntity,
                    err.to_string(),
                ));
            }
            AddFileToCollectionError::Error(err) => {
                let body = body.into_inner();
                log::error!(target: "routes::collection::controllers", controller = "add_file_to_collection", service = "CollectionFilePairService", collection_id:serde, body:serde, err:err; "Error returned from service.");
                return Err(Status::InternalServerError.into());
            }
        },
    };

    Ok((Status::Created, Json(pair)))
}

/// TODO: add a test for this route
#[delete("/<collection_id>/files/<file_id>")]
async fn remove_file_from_collection(
    #[allow(unused_variables)] sess: AuthUserSession<'_>,
    collection_file_pair_service: &State<Arc<CollectionFilePairService>>,
    collection_id: Uuid,
    file_id: Uuid,
) -> JsonRes<Option<CollectionFilePair>> {
    let pair = collection_file_pair_service
        .remove_file_from_collection(collection_id, file_id)
        .await;

    let pair = match pair {
        Ok(pair) => pair,
        Err(err) => match err {
            RemoveFileFromCollectionError::InvalidCollection { .. } => {
                return Err(Error::new_dynamic(Status::NotFound, err.to_string()));
            }
            RemoveFileFromCollectionError::InvalidFile { .. } => {
                return Err(Error::new_dynamic(Status::NotFound, err.to_string()));
            }
            RemoveFileFromCollectionError::Error(err) => {
                log::error!(target: "routes::collection::controllers", controller = "remove_file_from_collection", service = "CollectionFilePairService", collection_id:serde, file_id:serde, err:err; "Error returned from service.");
                return Err(Status::InternalServerError.into());
            }
        },
    };

    Ok((Status::Ok, Json(pair)))
}
