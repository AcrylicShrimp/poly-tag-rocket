mod staging_file_remover;

pub use staging_file_remover::*;

use crate::{
    config::AppConfig,
    services::{StagingFileService, UserService},
};
use chrono::Duration;
use rocket::{Build, Rocket};
use std::sync::Arc;

pub fn register_fairings(rocket: Rocket<Build>, app_config: &AppConfig) -> Rocket<Build> {
    let staging_file_service = rocket.state::<Arc<StagingFileService>>().unwrap();
    let user_service = rocket.state::<Arc<UserService>>().unwrap();

    let staging_file_remover = StagingFileRemover::new(
        Duration::new(app_config.expired_staging_file_removal_period as i64, 0).unwrap(),
        Duration::new(app_config.expired_staging_file_expiration as i64, 0).unwrap(),
        staging_file_service.clone(),
    );

    rocket.attach(staging_file_remover)
}
