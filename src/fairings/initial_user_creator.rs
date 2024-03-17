use crate::{config::AppConfig, services::UserService};
use rocket::{
    fairing::{Fairing, Info},
    Orbit, Rocket,
};
use std::sync::Arc;

pub struct InitialUserCreator;

impl InitialUserCreator {
    pub fn new() -> Self {
        InitialUserCreator
    }
}

#[rocket::async_trait]
impl Fairing for InitialUserCreator {
    fn info(&self) -> Info {
        Info {
            name: "Initial User Creator",
            kind: rocket::fairing::Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let app_config = rocket.state::<AppConfig>().unwrap();
        let user_service = rocket.state::<Arc<UserService>>().unwrap();

        let initial_user_config = match &app_config.initial_user {
            Some(user) => {
                log::info!(target: "fairings::initial_user_creator", method = "on_liftoff", fairing = "InitialUserCreator"; "Initial user configuration found.");
                user
            }
            None => {
                log::info!(target: "fairings::initial_user_creator", method = "on_liftoff", fairing = "InitialUserCreator"; "Initial user configuration not found. Skipping.");
                return;
            }
        };

        let initial_user = user_service
            .get_user_by_email(&initial_user_config.email)
            .await;
        let initial_user = match initial_user {
            Ok(user) => user,
            Err(err) => {
                log::warn!(target: "fairings::initial_user_creator", method = "on_liftoff", fairing = "InitialUserCreator", service = "UserService", err:err; "Error returned when attempting to get initial user. Aborting.");
                return;
            }
        };

        match initial_user {
            Some(initial_user) => {
                log::info!(target: "fairings::initial_user_creator", method = "on_liftoff", fairing = "InitialUserCreator"; "Initial user already exists. Updating.");

                let result = user_service
                    .set_user_username_by_id(initial_user.id, &initial_user_config.username)
                    .await;

                if let Err(err) = result {
                    log::warn!(target: "fairings::initial_user_creator", method = "on_liftoff", fairing = "InitialUserCreator", service = "UserService", err:err; "Error returned when attempting to update username of initial user.");
                }

                let result = user_service
                    .set_user_password_by_id(initial_user.id, &initial_user_config.password)
                    .await;

                if let Err(err) = result {
                    log::warn!(target: "fairings::initial_user_creator", method = "on_liftoff", fairing = "InitialUserCreator", service = "UserService", err:err; "Error returned when attempting to update password of initial user.");
                }
            }
            None => {
                log::info!(target: "fairings::initial_user_creator", method = "on_liftoff", fairing = "InitialUserCreator"; "Initial user does not exist. Creating.");

                let result = user_service
                    .create_user(
                        &initial_user_config.username,
                        &initial_user_config.email,
                        &initial_user_config.password,
                    )
                    .await;

                if let Err(err) = result {
                    log::warn!(target: "fairings::initial_user_creator", method = "on_liftoff", fairing = "InitialUserCreator", service = "UserService", err:err; "Error returned when attempting to create initial user.");
                }
            }
        }

        log::info!(target: "fairings::initial_user_creator", method = "on_liftoff", fairing = "InitialUserCreator"; "Initial user is ready.");
    }
}
