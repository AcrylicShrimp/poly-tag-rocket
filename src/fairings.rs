mod initial_user_creator;
mod staging_file_remover;

pub use initial_user_creator::*;
pub use staging_file_remover::*;

use crate::config::AppConfig;
use chrono::Duration;
use rocket::{Build, Rocket};

pub fn register_fairings(rocket: Rocket<Build>, app_config: &AppConfig) -> Rocket<Build> {
    let staging_file_remover = StagingFileRemover::new(
        Duration::new(app_config.expired_staging_file_removal_period as i64, 0).unwrap(),
        Duration::new(app_config.expired_staging_file_expiration as i64, 0).unwrap(),
    );
    let initial_user_creator = InitialUserCreator::new();

    rocket
        .attach(staging_file_remover)
        .attach(initial_user_creator)
}
