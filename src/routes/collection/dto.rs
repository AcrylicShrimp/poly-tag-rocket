use crate::db::models::Collection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct CreatingCollection<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
}

#[derive(Serialize, Deserialize)]
pub struct AddingCollectionFile {
    pub file_id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct SearchingCollection<'a> {
    pub query: &'a str,
}

#[derive(Serialize, Deserialize)]
pub struct UpdatingCollection<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
}

#[derive(Serialize, Deserialize)]
pub struct CollectionSearchResult {
    pub collections: Vec<Collection>,
}

#[derive(Serialize, Deserialize)]
pub struct CollectionList {
    pub collections: Vec<Collection>,
    pub last_collection_id: Option<Uuid>,
    pub limit: u32,
}
