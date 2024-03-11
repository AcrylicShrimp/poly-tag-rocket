use super::file_driver::FileDriver;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use std::sync::Arc;

pub struct FileService {
    db_pool: Pool<AsyncPgConnection>,
    file_driver: Box<dyn FileDriver + Send + Sync>,
}

impl FileService {
    pub fn new(
        db_pool: Pool<AsyncPgConnection>,
        file_driver: Box<impl 'static + FileDriver + Send + Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            db_pool,
            file_driver: file_driver as Box<dyn FileDriver + Send + Sync>,
        })
    }
}
