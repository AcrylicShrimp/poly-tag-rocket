use super::SearchService;
use crate::db::models::{CollectionFilePair, CreatingCollectionFilePair, File};
use diesel::{BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum CollectionFilePairServiceError {
    #[error("database pool error: {0}")]
    Pool(#[from] diesel_async::pooled_connection::deadpool::PoolError),
    #[error("diesel error: {0}")]
    Diesel(#[from] diesel::result::Error),
}

#[derive(Error, Debug)]
pub enum AddFileToCollectionError {
    #[error("collection with ID `{collection_id}` already contains file with ID `{file_id}`")]
    AlreadyExists { collection_id: Uuid, file_id: Uuid },
    #[error("collection with ID `{collection_id}` does not exist")]
    InvalidCollection { collection_id: Uuid },
    #[error("file with ID `{file_id}` does not exist")]
    InvalidFile { file_id: Uuid },
    #[error("{0}")]
    Error(#[from] CollectionFilePairServiceError),
}

#[derive(Error, Debug)]
pub enum RemoveFileFromCollectionError {
    #[error("collection with ID `{collection_id}` does not exist")]
    InvalidCollection { collection_id: Uuid },
    #[error("file with ID `{file_id}` does not exist")]
    InvalidFile { file_id: Uuid },
    #[error("{0}")]
    Error(#[from] CollectionFilePairServiceError),
}

pub struct CollectionFilePairService {
    db_pool: Pool<AsyncPgConnection>,
    search_service: Arc<SearchService>,
}

impl CollectionFilePairService {
    pub fn new(db_pool: Pool<AsyncPgConnection>, search_service: Arc<SearchService>) -> Arc<Self> {
        Arc::new(Self {
            db_pool,
            search_service,
        })
    }

    /// Adds a file to a collection.
    pub async fn add_file_to_collection(
        &self,
        collection_id: Uuid,
        file_id: Uuid,
    ) -> Result<CollectionFilePair, AddFileToCollectionError> {
        use crate::db::schema;

        let db = &mut self
            .db_pool
            .get()
            .await
            .map_err(CollectionFilePairServiceError::from)?;

        let file = schema::files::dsl::files
            .select((
                schema::files::id,
                schema::files::name,
                schema::files::mime,
                schema::files::size,
                schema::files::hash,
                schema::files::uploaded_at,
            ))
            .filter(schema::files::id.eq(file_id))
            .get_result::<File>(db)
            .await
            .optional()
            .map_err(CollectionFilePairServiceError::from)?;

        let file = match file {
            Some(file) => file,
            None => return Err(AddFileToCollectionError::InvalidFile { file_id }),
        };

        let pair = diesel::insert_into(schema::collection_file_pairs::table)
            .values(CreatingCollectionFilePair {
                collection_id,
                file_id,
            })
            .returning((
                schema::collection_file_pairs::collection_id,
                schema::collection_file_pairs::file_id,
            ))
            .get_result::<CollectionFilePair>(db)
            .await;

        let pair = match pair {
            Ok(pair) => pair,
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            )) => {
                return Err(AddFileToCollectionError::AlreadyExists {
                    collection_id,
                    file_id,
                })
            }
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                err,
            )) if err.constraint_name() == Some("collection_fk") => {
                return Err(AddFileToCollectionError::InvalidCollection { collection_id })
            }
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                err,
            )) if err.constraint_name() == Some("file_fk") => {
                return Err(AddFileToCollectionError::InvalidFile { file_id })
            }
            Err(err) => return Err(CollectionFilePairServiceError::from(err).into()),
        };

        // ignore the error if the indexing fails, as it is not critical
        self.search_service
            .index_collection_file(collection_id, &file)
            .await
            .ok();

        Ok(pair)
    }

    /// Removes a file from a collection.
    /// Returns the pair that was removed, or `None` if no pair was found.
    pub async fn remove_file_from_collection(
        &self,
        collection_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<CollectionFilePair>, RemoveFileFromCollectionError> {
        use crate::db::schema;

        let db = &mut self
            .db_pool
            .get()
            .await
            .map_err(CollectionFilePairServiceError::from)?;

        let pair = diesel::delete(
            schema::collection_file_pairs::dsl::collection_file_pairs.filter(
                schema::collection_file_pairs::collection_id
                    .eq(collection_id)
                    .and(schema::collection_file_pairs::file_id.eq(file_id)),
            ),
        )
        .returning((
            schema::collection_file_pairs::collection_id,
            schema::collection_file_pairs::file_id,
        ))
        .get_result::<CollectionFilePair>(db)
        .await
        .optional();

        let pair = match pair {
            Ok(pair) => pair,
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                err,
            )) if err.constraint_name() == Some("collection_fk") => {
                return Err(RemoveFileFromCollectionError::InvalidCollection { collection_id })
            }
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                err,
            )) if err.constraint_name() == Some("file_fk") => {
                return Err(RemoveFileFromCollectionError::InvalidFile { file_id })
            }
            Err(err) => return Err(CollectionFilePairServiceError::from(err).into()),
        };

        if pair.is_some() {
            // ignore the error if the indexing fails, as it is not critical
            self.search_service
                .remove_collection_file(collection_id, file_id)
                .await
                .ok();
        }

        Ok(pair)
    }

    /// Retrieves a list of files in a collection.
    /// The result will be sorted by name and ID (name first) in ascending order.
    /// If `last_file_id` is provided, the result will start from the file that comes after it.
    pub async fn get_files_in_collection(
        &self,
        collection_id: Uuid,
        last_file_id: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<File>, CollectionFilePairServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;

        let query = schema::collection_file_pairs::table
            .inner_join(schema::files::table)
            .filter(schema::collection_file_pairs::collection_id.eq(collection_id))
            .select((
                schema::files::id,
                schema::files::name,
                schema::files::mime,
                schema::files::size,
                schema::files::hash,
                schema::files::uploaded_at,
            ))
            .order((schema::files::name.asc(), schema::files::id.asc()))
            .limit(limit as i64);

        let last_file = match last_file_id {
            Some(last_file_id) => {
                let last_file = schema::collection_file_pairs::table
                    .inner_join(schema::files::table)
                    .select((schema::files::name, schema::files::id))
                    .filter(
                        schema::collection_file_pairs::collection_id
                            .eq(collection_id)
                            .and(schema::files::id.eq(last_file_id)),
                    )
                    .get_result::<(String, Uuid)>(db)
                    .await
                    .optional()?;

                let last_file = match last_file {
                    Some(pair) => pair,
                    None => return Ok(Vec::new()),
                };

                Some(last_file)
            }
            None => None,
        };

        let files = match &last_file {
            Some((last_file_name, last_file_id)) => query
                .filter(
                    schema::files::name
                        .gt(last_file_name)
                        .or(schema::files::name
                            .eq(last_file_name)
                            .and(schema::files::id.gt(last_file_id))),
                )
                .load::<File>(db),
            None => query.load::<File>(db),
        };
        let files = files.await?;

        Ok(files)
    }

    /// Retrieves a file by its ID.
    pub async fn get_file_in_collection_by_id(
        &self,
        collection_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<File>, CollectionFilePairServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let file = schema::collection_file_pairs::table
            .inner_join(schema::files::table)
            .filter(
                schema::collection_file_pairs::collection_id
                    .eq(collection_id)
                    .and(schema::collection_file_pairs::file_id.eq(file_id)),
            )
            .select((
                schema::files::id,
                schema::files::name,
                schema::files::mime,
                schema::files::size,
                schema::files::hash,
                schema::files::uploaded_at,
            ))
            .get_result::<File>(db)
            .await
            .optional()?;

        Ok(file)
    }
}
