mod auth_service;
mod collection_file_pair_service;
mod collection_service;
mod file_driver;
mod file_service;
mod metric_service;
mod password_service;
mod search_service;
mod staging_file_service;
mod user_service;

pub use auth_service::*;
pub use collection_file_pair_service::*;
pub use collection_service::*;
pub use file_driver::*;
pub use file_service::*;
pub use metric_service::*;
pub use password_service::*;
pub use search_service::*;
pub use staging_file_service::*;
pub use user_service::*;

use crate::config::AppConfig;
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use rocket::{Build, Rocket};
use std::{path::PathBuf, sync::Arc};

pub async fn register_search_service(
    rocket: Rocket<Build>,
    app_config: &AppConfig,
) -> Result<Rocket<Build>, SearchServiceError> {
    let search_service = SearchService::new(
        &app_config.meilisearch_url,
        app_config.meilisearch_master_key.as_deref(),
        app_config.meilisearch_index_prefix.as_deref(),
    )
    .await?;

    Ok(rocket.manage(search_service))
}

pub fn register_services(
    rocket: Rocket<Build>,
    db_pool: Pool<AsyncPgConnection>,
    file_base_path: impl Into<PathBuf>,
    file_driver: Arc<impl 'static + FileDriver + Send + Sync>,
) -> Rocket<Build> {
    let search_service = rocket.state::<Arc<SearchService>>().unwrap();

    let password_service = PasswordService::new();
    let auth_service = AuthService::new(db_pool.clone(), password_service.clone());
    let collection_service = CollectionService::new(db_pool.clone(), search_service.clone());
    let staging_file_service = StagingFileService::new(db_pool.clone(), file_driver.clone());
    let file_service = FileService::new(
        db_pool.clone(),
        staging_file_service.clone(),
        search_service.clone(),
        file_driver,
    );
    let user_service = UserService::new(db_pool, password_service.clone());
    let metric_service = MetricService::new(file_base_path);

    rocket
        .manage(password_service)
        .manage(auth_service)
        .manage(collection_service)
        .manage(staging_file_service)
        .manage(file_service)
        .manage(user_service)
        .manage(metric_service)
}
