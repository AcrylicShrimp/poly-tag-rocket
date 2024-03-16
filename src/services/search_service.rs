use crate::db::models::Collection;
use meilisearch_sdk::{Client, Index};
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

                match task.try_make_index(&client) {
                    Ok(index) => index,
                    Err(_) => {
                        log::error!(target: "search_service", collections_index_name; "Failed to get index. Aborting.");
                        return Err(SearchServiceError::IndexInTaskNotFound);
                    }
                }
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

                match task.try_make_index(&client) {
                    Ok(index) => index,
                    Err(_) => {
                        log::error!(target: "search_service", files_index_name; "Failed to get index. Aborting.");
                        return Err(SearchServiceError::IndexInTaskNotFound);
                    }
                }
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

    /// Indexes a file.
    /// It will overwrite the previous with the same ID.
    pub async fn index_file(
        &self,
        file: &crate::db::models::File,
    ) -> Result<(), SearchServiceError> {
        let result = self.files_index.add_or_replace(&[file], Some("id")).await;

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
