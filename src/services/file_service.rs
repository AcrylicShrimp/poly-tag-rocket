use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use std::sync::Arc;

pub struct FileService {
    db_pool: Pool<AsyncPgConnection>,
}

impl FileService {
    pub fn new(db_pool: Pool<AsyncPgConnection>) -> Arc<Self> {
        Arc::new(Self { db_pool })
    }
}
