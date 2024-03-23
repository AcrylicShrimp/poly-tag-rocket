use super::SearchService;
use crate::db::models::{Collection, CreatingCollection, UpdatingCollection};
use chrono::NaiveDateTime;
use diesel::{BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum CollectionServiceError {
    #[error("database pool error: {0}")]
    Pool(#[from] diesel_async::pooled_connection::deadpool::PoolError),
    #[error("diesel error: {0}")]
    Diesel(#[from] diesel::result::Error),
}

pub struct CollectionService {
    db_pool: Pool<AsyncPgConnection>,
    search_service: Arc<SearchService>,
}

impl CollectionService {
    pub fn new(db_pool: Pool<AsyncPgConnection>, search_service: Arc<SearchService>) -> Arc<Self> {
        Arc::new(Self {
            db_pool,
            search_service,
        })
    }

    /// Creates a new collection.
    pub async fn create_collection(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> Result<Collection, CollectionServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let collection = diesel::insert_into(schema::collections::table)
            .values(CreatingCollection { name, description })
            .returning((
                schema::collections::id,
                schema::collections::name,
                schema::collections::description,
                schema::collections::created_at,
            ))
            .get_result::<Collection>(db)
            .await?;

        // ignore the error if the indexing fails, as it is not critical
        self.search_service.index_collection(&collection).await.ok();

        Ok(collection)
    }

    /// Removes a collection by its ID.
    /// Returns the collection that was removed, or `None` if no collection was found.
    pub async fn remove_collection_by_id(
        &self,
        collection_id: Uuid,
    ) -> Result<Option<Collection>, CollectionServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let collection = diesel::delete(
            schema::collections::dsl::collections.filter(schema::collections::id.eq(collection_id)),
        )
        .returning((
            schema::collections::id,
            schema::collections::name,
            schema::collections::description,
            schema::collections::created_at,
        ))
        .get_result::<Collection>(db)
        .await
        .optional()?;

        if collection.is_some() {
            // ignore the error if the indexing fails, as it is not critical
            self.search_service
                .remove_collection_by_id(collection_id)
                .await
                .ok();
        }

        Ok(collection)
    }

    /// Retrieves a list of collections.
    /// The result will be sorted by name and ID (name first) in ascending order.
    /// If `last_collection_id` is provided, the result will start from the collection that comes after it.
    pub async fn get_collections(
        &self,
        last_collection_id: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<Collection>, CollectionServiceError> {
        use crate::db::schema;
        let db = &mut self.db_pool.get().await?;

        let query = schema::collections::dsl::collections
            .select((
                schema::collections::id,
                schema::collections::name,
                schema::collections::description,
                schema::collections::created_at,
            ))
            .order((
                schema::collections::created_at.asc(),
                schema::collections::id.asc(),
            ))
            .limit(limit as i64);

        let collections = match last_collection_id {
            Some(last_collection_id) => {
                let last_collection = schema::collections::dsl::collections
                    .select((schema::collections::created_at, schema::collections::id))
                    .filter(schema::collections::id.eq(last_collection_id))
                    .first::<(NaiveDateTime, Uuid)>(db)
                    .await
                    .optional()?;

                let (last_collection_created_at, last_collection_id) = match last_collection {
                    Some((created_at, id)) => (created_at, id),
                    None => return Ok(Vec::new()),
                };

                query
                    .filter(
                        schema::collections::created_at
                            .gt(last_collection_created_at)
                            .or(schema::collections::created_at
                                .eq(last_collection_created_at)
                                .and(schema::collections::id.gt(last_collection_id))),
                    )
                    .load::<Collection>(db)
            }
            None => query.load::<Collection>(db),
        };
        let collections = collections.await?;

        Ok(collections)
    }

    /// Retrieves a collection by its ID.
    pub async fn get_collection_by_id(
        &self,
        collection_id: Uuid,
    ) -> Result<Option<Collection>, CollectionServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let collection = schema::collections::dsl::collections
            .filter(schema::collections::id.eq(collection_id))
            .select((
                schema::collections::id,
                schema::collections::name,
                schema::collections::description,
                schema::collections::created_at,
            ))
            .first::<Collection>(db)
            .await
            .optional()?;

        Ok(collection)
    }

    /// Updates a collection by its ID.
    /// Returns the collection that was updated, or `None` if no collection was found.
    pub async fn update_collection_by_id(
        &self,
        collection_id: Uuid,
        new_name: &str,
        new_description: Option<&str>,
    ) -> Result<Option<Collection>, CollectionServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let collection = diesel::update(
            schema::collections::dsl::collections.filter(schema::collections::id.eq(collection_id)),
        )
        .set(UpdatingCollection {
            name: new_name,
            description: new_description,
        })
        .returning((
            schema::collections::id,
            schema::collections::name,
            schema::collections::description,
            schema::collections::created_at,
        ))
        .get_result::<Collection>(db)
        .await
        .optional()?;

        if let Some(collection) = &collection {
            // ignore the error if the indexing fails, as it is not critical
            self.search_service.index_collection(collection).await.ok();
        }

        Ok(collection)
    }
}
