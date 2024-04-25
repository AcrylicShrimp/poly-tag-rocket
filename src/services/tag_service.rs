use super::{FileService, SearchService};
use crate::db::models::CreatingTag;
use diesel::{
    expression::AsExpression, sql_types::Bool, BoolExpressionMethods, BoxableExpression,
    ExpressionMethods, QueryDsl,
};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum TagServiceError {
    #[error("database pool error: {0}")]
    PoolError(#[from] diesel_async::pooled_connection::deadpool::PoolError),
    #[error("diesel error: {0}")]
    DieselError(#[from] diesel::result::Error),
}

#[derive(Error, Debug)]
pub enum AddTagToFileError<'a> {
    #[error("some of the file does not exist: `{file_ids:?}`")]
    InvalidFiles { file_ids: &'a [Uuid] },
    #[error("{0}")]
    Error(#[from] TagServiceError),
}

#[derive(Error, Debug)]
pub enum RemoveTagFromFileError<'a> {
    #[error("some of the file does not exist: `{file_ids:?}`")]
    InvalidFiles { file_ids: &'a [Uuid] },
    #[error("{0}")]
    Error(#[from] TagServiceError),
}

pub struct TagService {
    db_pool: Pool<AsyncPgConnection>,
    file_service: Arc<FileService>,
    search_service: Arc<SearchService>,
}

impl TagService {
    pub fn new(
        db_pool: Pool<AsyncPgConnection>,
        file_service: Arc<FileService>,
        search_service: Arc<SearchService>,
    ) -> Arc<Self> {
        Arc::new(Self {
            db_pool,
            file_service,
            search_service,
        })
    }

    pub async fn add_tags_to_files<'a>(
        &self,
        file_ids: &'a [Uuid],
        tags: &'a [impl AsRef<str>],
    ) -> Result<usize, AddTagToFileError<'a>> {
        use crate::db::schema;

        let mut creating_tags = Vec::with_capacity(file_ids.len() * tags.len());

        for &file_id in file_ids {
            for tag in tags {
                creating_tags.push(CreatingTag {
                    name: tag.as_ref(),
                    file_id,
                });
            }
        }

        let db = &mut self.db_pool.get().await.map_err(TagServiceError::from)?;

        let result = diesel::insert_into(schema::tags::table)
            .values(creating_tags)
            .on_conflict_do_nothing()
            .execute(db)
            .await;

        let count = match result {
            Ok(count) => count,
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                err,
            )) if err.constraint_name() == Some("tags_file_fk") => {
                return Err(AddTagToFileError::InvalidFiles { file_ids });
            }
            Err(err) => return Err(TagServiceError::from(err).into()),
        };

        // TODO: index the tags

        Ok(count)
    }

    pub async fn remove_tags_from_files<'a>(
        &self,
        file_ids: &'a [Uuid],
        tags: &'a [impl AsRef<str>],
    ) -> Result<usize, RemoveTagFromFileError<'a>> {
        use crate::db::schema;

        if file_ids.is_empty() || tags.is_empty() {
            return Ok(0);
        }

        let db = &mut self.db_pool.get().await.map_err(TagServiceError::from)?;

        let mut conditions: Box<dyn BoxableExpression<_, _, SqlType = _>> =
            Box::new(<bool as AsExpression<Bool>>::as_expression(false));

        for &file_id in file_ids {
            for tag in tags {
                conditions = Box::new(
                    conditions.or(schema::tags::name
                        .eq(tag.as_ref())
                        .and(schema::tags::file_id.eq(file_id))),
                );
            }
        }

        let result = diesel::delete(schema::tags::table.filter(conditions))
            .execute(db)
            .await;

        let count = match result {
            Ok(count) => count,
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                err,
            )) if err.constraint_name() == Some("tags_file_fk") => {
                return Err(RemoveTagFromFileError::InvalidFiles { file_ids });
            }
            Err(err) => return Err(TagServiceError::from(err).into()),
        };

        // TODO: index the tags

        Ok(count)
    }
}
