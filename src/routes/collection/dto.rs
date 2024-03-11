use crate::db::models::Collection;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CreatingCollection<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdatingCollection<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
}

#[derive(Serialize, Deserialize)]
pub struct CollectionList {
    pub collections: Vec<Collection>,
    pub last_collection_id: Option<i32>,
    pub limit: u32,
}
