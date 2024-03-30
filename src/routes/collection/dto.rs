use crate::db::models::{Collection, File};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct CreatingCollection<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
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

#[derive(Serialize, Deserialize)]
pub struct AddingCollectionFile {
    pub file_id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct SearchingCollectionFile<'a> {
    pub query: &'a str,
    pub filter_mime: Option<&'a str>,
    pub filter_size: Option<(u32, u32)>,
    pub filter_hash: Option<u32>,
    pub filter_uploaded_at: Option<(NaiveDateTime, NaiveDateTime)>,
}

#[derive(Serialize, Deserialize)]
pub struct CollectionFileSearchResult {
    pub files: Vec<File>,
}

#[derive(Serialize, Deserialize)]
pub struct CollectionFileList {
    pub files: Vec<File>,
    pub last_file_id: Option<Uuid>,
    pub limit: u32,
}
