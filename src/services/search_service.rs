use crate::db::models::{Collection, File};
use chrono::{DateTime, NaiveDateTime};
use meilisearch_sdk::{Client, Index, Selectors};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum SearchServiceError {
    #[error("meilisearch error: {0}")]
    MeiliSearchError(#[from] meilisearch_sdk::errors::Error),
    #[error("index not found in task")]
    IndexInTaskNotFound,
}

#[derive(Serialize)]
struct IndexingFile<'a> {
    pub id: Uuid,
    pub name: &'a str,
    pub mime_full: &'a str,
    pub mime_type_part: &'a str,
    pub mime_subtype_part: Option<&'a str>,
    pub size: i64,
    pub hash: i64,
    pub uploaded_at: i64,
}

impl<'a> IndexingFile<'a> {
    pub fn from_file(file: &'a File) -> Self {
        let (mime_type_part, mime_subtype_part) = match file.mime.trim().split_once('/') {
            Some((type_part, subtype_part)) => (type_part, Some(subtype_part)),
            None => (file.mime.as_str(), None),
        };

        let uploaded_at = file.uploaded_at.and_utc().timestamp_micros();

        Self {
            id: file.id,
            name: &file.name,
            mime_full: &file.mime,
            mime_type_part: mime_type_part,
            mime_subtype_part: mime_subtype_part,
            size: file.size,
            hash: file.hash,
            uploaded_at,
        }
    }
}

#[derive(Deserialize)]
struct IndexedFile {
    pub id: Uuid,
    pub name: String,
    pub mime_full: String,
    pub size: i64,
    pub hash: i64,
    pub uploaded_at: i64,
}

impl IndexedFile {
    pub fn into_file(self) -> File {
        let uploaded_at = DateTime::from_timestamp_micros(self.uploaded_at).unwrap();
        let uploaded_at = uploaded_at.naive_utc();

        File {
            id: self.id,
            name: self.name,
            mime: self.mime_full,
            size: self.size,
            hash: self.hash,
            uploaded_at,
        }
    }
}

pub struct SearchService {
    collections_index: Index,
    files_index: Index,
}

impl SearchService {
    pub async fn new(
        meilisearch_url: &str,
        meilisearch_master_key: Option<&str>,
        meilisearch_index_prefix: Option<&str>,
    ) -> Result<Arc<Self>, SearchServiceError> {
        let meilisearch_url: &str = meilisearch_url.trim_end_matches('/');
        let meilisearch_index_prefix = match meilisearch_index_prefix {
            Some(prefix) => format!("{}_", prefix.to_ascii_lowercase()),
            None => String::new(),
        };
        let client = Client::new(meilisearch_url, meilisearch_master_key);

        fn make_index_name(index_prefix: &str, name: &str) -> String {
            format!("{}{}", index_prefix, name)
        }

        let collections_index_name = make_index_name(&meilisearch_index_prefix, "collections");
        let files_index_name = make_index_name(&meilisearch_index_prefix, "files");

        log::info!(target: "search_service", collections_index_name, files_index_name; "Creating indices. It may produce warnings if the indices are not found.");

        let collections_index = client.get_index(&collections_index_name).await;
        let files_index = client.get_index(&files_index_name).await;

        let collections_index = match collections_index {
            Ok(index) => {
                log::info!(target: "search_service", collections_index_name; "Index already exists. Skipping creation.");
                index
            }
            // ignore the error, assuming it's because the index doesn't exist
            Err(_) => {
                let task = client
                    .create_index(&collections_index_name, Some("id"))
                    .await;
                let task = match task {
                    Ok(task) => task,
                    Err(err) => {
                        log::error!(target: "search_service", collections_index_name, err:err; "Failed to create index. Aborting.");
                        return Err(err.into());
                    }
                };

                let task = task.wait_for_completion(&client, None, None).await;
                let task = match task {
                    Ok(task) => task,
                    Err(err) => {
                        log::error!(target: "search_service", collections_index_name, err:err; "Failed to wait for index creation. Aborting.");
                        return Err(err.into());
                    }
                };

                let index = match task.try_make_index(&client) {
                    Ok(index) => index,
                    Err(_) => {
                        log::error!(target: "search_service", collections_index_name; "Failed to get index. Aborting.");
                        return Err(SearchServiceError::IndexInTaskNotFound);
                    }
                };

                if let Err(err) = index
                    .set_searchable_attributes(["name", "description"])
                    .await
                {
                    // failing to set searchable attributes is not a critical error
                    log::warn!(target: "search_service", collections_index_name, err:err; "Failed to set searchable attributes.");
                }

                if let Err(err) = index.set_filterable_attributes(["created_at"]).await {
                    // failing to set searchable attributes is not a critical error
                    log::warn!(target: "search_service", collections_index_name, err:err; "Failed to set searchable attributes.");
                }

                index
            }
        };

        let files_index = match files_index {
            Ok(index) => {
                log::info!(target: "search_service", files_index_name; "Index already exists. Skipping creation.");
                index
            }
            // ignore the error, assuming it's because the index doesn't exist
            Err(_) => {
                let task = client.create_index(&files_index_name, Some("id")).await;
                let task = match task {
                    Ok(task) => task,
                    Err(err) => {
                        log::error!(target: "search_service", files_index_name, err:err; "Failed to create index. Aborting.");
                        return Err(err.into());
                    }
                };

                let task = task.wait_for_completion(&client, None, None).await;
                let task = match task {
                    Ok(task) => task,
                    Err(err) => {
                        log::error!(target: "search_service", files_index_name, err:err; "Failed to wait for index creation. Aborting.");
                        return Err(err.into());
                    }
                };

                let index = match task.try_make_index(&client) {
                    Ok(index) => index,
                    Err(_) => {
                        log::error!(target: "search_service", files_index_name; "Failed to get index. Aborting.");
                        return Err(SearchServiceError::IndexInTaskNotFound);
                    }
                };

                if let Err(err) = index.set_searchable_attributes(["name"]).await {
                    // failing to set searchable attributes is not a critical error
                    log::warn!(target: "search_service", files_index_name, err:err; "Failed to set searchable attributes.");
                }

                if let Err(err) = index
                    .set_filterable_attributes([
                        "mime_full",
                        "mime_type_part",
                        "mime_subtype_part",
                        "size",
                        "hash",
                        "uploaded_at",
                    ])
                    .await
                {
                    // failing to set filterable attributes is not a critical error
                    log::warn!(target: "search_service", files_index_name, err:err; "Failed to set filterable attributes.");
                }

                index
            }
        };

        Ok(Arc::new(Self {
            collections_index,
            files_index,
        }))
    }

    /// Indexes a collection.
    /// It will overwrite the previous with the same ID.
    pub async fn index_collection(
        &self,
        collection: &Collection,
    ) -> Result<(), SearchServiceError> {
        let result = self
            .collections_index
            .add_or_replace(&[collection], Some("id"))
            .await;

        if let Err(err) = result {
            let index_uid = &self.collections_index.uid;
            log::error!(target: "search_service", index_uid, collection:serde, err:err; "Failed to add a collection to index.");
            return Err(err.into());
        }

        Ok(())
    }

    /// Removes a collection from the index.
    /// /// It will not fail if the collection is not found in the index.
    pub async fn remove_collection_by_id(
        &self,
        collection_id: Uuid,
    ) -> Result<(), SearchServiceError> {
        if let Err(err) = self.collections_index.delete_document(collection_id).await {
            let index_uid = &self.collections_index.uid;
            log::error!(target: "search_service", index_uid, collection_id:serde, err:err; "Failed to remove collection.");
        }

        Ok(())
    }

    /// Searches collections.
    pub async fn search_collections(&self, q: &str) -> Result<Vec<Collection>, SearchServiceError> {
        let query = self.collections_index.search().with_query(q).build();

        let result = query.execute::<Collection>().await;
        let result = match result {
            Ok(result) => result,
            Err(err) => {
                let index_uid = &self.collections_index.uid;
                log::error!(target: "search_service", index_uid, q, err:err; "Failed to search collections.");
                return Err(err.into());
            }
        };

        let hits = result.hits.into_iter().map(|hit| hit.result).collect();

        Ok(hits)
    }

    /// Indexes a file.
    /// It will overwrite the previous with the same ID.
    pub async fn index_file(&self, file: &File) -> Result<(), SearchServiceError> {
        let indexing_file = IndexingFile::from_file(file);

        let result = self
            .files_index
            .add_or_replace(&[indexing_file], Some("id"))
            .await;

        if let Err(err) = result {
            let index_uid = &self.files_index.uid;
            log::error!(target: "search_service", index_uid, file:serde, err:err; "Failed to add a file to index.");
            return Err(err.into());
        }

        Ok(())
    }

    /// Removes a file from the index.
    /// It will not fail if the file is not found in the index.
    pub async fn remove_file_by_id(&self, file_id: Uuid) -> Result<(), SearchServiceError> {
        if let Err(err) = self.files_index.delete_document(file_id).await {
            let index_uid = &self.files_index.uid;
            log::error!(target: "search_service", index_uid, file_id:serde, err:err; "Failed to remove file.");
        }

        Ok(())
    }

    /// Searches files.
    pub async fn search_files(
        &self,
        q: &str,
        filter_mime: Option<&str>,
        filter_size: Option<(u32, u32)>,
        filter_hash: Option<u32>,
        filter_uploaded_at: Option<(NaiveDateTime, NaiveDateTime)>,
    ) -> Result<Vec<File>, SearchServiceError> {
        let mut array_filter = Vec::with_capacity(4);

        if let Some(filter_mime) = filter_mime {
            array_filter.push(format!(
                "mime_full = \"{}\" OR mime_type_part = \"{}\" OR mime_subtype_part = \"{}\"",
                filter_mime, filter_mime, filter_mime
            ));
        }

        if let Some(filter_size) = filter_size {
            array_filter.push(format!("size {} TO {}", filter_size.0, filter_size.1));
        }

        if let Some(filter_hash) = filter_hash {
            array_filter.push(format!("hash = {}", filter_hash));
        }

        if let Some(filter_uploaded_at) = filter_uploaded_at {
            let start_timestamp = filter_uploaded_at.0.and_utc().timestamp();
            let end_timestamp = filter_uploaded_at.1.and_utc().timestamp();

            array_filter.push(format!(
                "uploaded_at {} TO {}",
                start_timestamp, end_timestamp
            ));
        }

        let array_filter = array_filter.iter().map(|s| s.as_str()).collect();

        let query = self
            .files_index
            .search()
            .with_query(q)
            .with_array_filter(array_filter)
            .with_attributes_to_retrieve(Selectors::Some(&[
                "id",
                "name",
                "mime_full",
                "size",
                "hash",
                "uploaded_at",
            ]))
            .build();

        let result = query.execute::<IndexedFile>().await;
        let result = match result {
            Ok(result) => result,
            Err(err) => {
                let index_uid = &self.files_index.uid;
                log::error!(target: "search_service", index_uid, q, err:err; "Failed to search files.");
                return Err(err.into());
            }
        };

        let hits = result
            .hits
            .into_iter()
            .map(|hit| hit.result.into_file())
            .collect();

        Ok(hits)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use rocket::futures::executor::block_on;

    pub struct IndexDropper {
        client: Client,
        index_prefix: String,
    }

    impl IndexDropper {
        pub fn new(
            url: &str,
            master_key: Option<impl AsRef<str>>,
            index_prefix: impl Into<String>,
        ) -> Self {
            let client = Client::new(url, master_key.as_ref().map(|key| key.as_ref()));
            let index_prefix = index_prefix.into();

            Self {
                client,
                index_prefix,
            }
        }

        async fn drop_async(&self) {
            let task = self
                .client
                .delete_index(format!("{}_collections", self.index_prefix))
                .await
                .unwrap();
            task.wait_for_completion(&self.client, None, None)
                .await
                .unwrap();

            let task = self
                .client
                .delete_index(format!("{}_files", self.index_prefix))
                .await
                .unwrap();
            task.wait_for_completion(&self.client, None, None)
                .await
                .unwrap();
        }
    }

    impl Drop for IndexDropper {
        fn drop(&mut self) {
            block_on(self.drop_async());
        }
    }
}
