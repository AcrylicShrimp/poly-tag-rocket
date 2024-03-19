use crate::db::models::{CollectionFilePair, CreatingCollectionFilePair};
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
}

impl CollectionFilePairService {
    pub fn new(db_pool: Pool<AsyncPgConnection>) -> Arc<Self> {
        Arc::new(Self { db_pool })
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

        // TODO: add the pair to the search index

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
            // TODO: remove the pair from the search index
        }

        Ok(pair)
    }

    pub async fn get_files_in_collection(
        &self,
        collection_id: Uuid,
        last_file_id: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<Uuid>, CollectionFilePairServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let query = schema::collection_file_pairs::table
            .inner_join(schema::files::table)
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

        let files = schema::collection_file_pairs::dsl::collection_file_pairs
            .select(schema::collection_file_pairs::file_id)
            .filter(schema::collection_file_pairs::collection_id.eq(collection_id))
            .load::<Uuid>(db)
            .await?;

        Ok(files)
    }
}
