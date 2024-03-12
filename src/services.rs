mod auth_service;
mod collection_service;
mod file_driver;
mod file_service;
mod password_service;
mod staging_file_service;
mod user_service;

pub use auth_service::*;
pub use collection_service::*;
pub use file_driver::*;
pub use file_service::*;
pub use password_service::*;
pub use staging_file_service::*;
pub use user_service::*;

use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use rocket::{Build, Rocket};
use std::sync::Arc;

pub fn register_services(
    rocket: Rocket<Build>,
    db_pool: Pool<AsyncPgConnection>,
    file_driver: Arc<impl 'static + FileDriver + Send + Sync>,
) -> Rocket<Build> {
    let password_service = PasswordService::new();
    let auth_service = AuthService::new(db_pool.clone(), password_service.clone());
    let collection_service = CollectionService::new(db_pool.clone());
    let staging_file_service = StagingFileService::new(db_pool.clone(), file_driver.clone());
    let file_service = FileService::new(db_pool.clone(), staging_file_service.clone(), file_driver);
    let user_service = UserService::new(db_pool, password_service.clone());

    rocket
        .manage(password_service)
        .manage(auth_service)
        .manage(collection_service)
        .manage(staging_file_service)
        .manage(file_service)
        .manage(user_service)
}
