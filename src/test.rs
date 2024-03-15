use crate::{
    config::AppConfig,
    create_rocket_instance,
    db::{self, test::DatabaseDropper},
    setup_rocket_instance,
};
use rocket::{Build, Rocket};
use std::path::PathBuf;
use uuid::Uuid;

/// Creates a new Rocket instance for testing.
/// It creates a new database for the test and runs the migrations.
pub async fn create_test_rocket_instance() -> (Rocket<Build>, DatabaseDropper) {
    let mut app_config = AppConfig::load(None as Option<PathBuf>).unwrap();

    let database_url_base = app_config.database_url_base.clone();
    let maintenance_database_name = app_config.maintenance_database_name.clone();
    let id = Uuid::new_v4().to_string();

    let database_name =
        db::test::create_test_database(&database_url_base, &maintenance_database_name, &id)
            .unwrap();
    app_config.database_name = database_name.clone();

    let rocket = create_rocket_instance(&app_config).unwrap();
    let rocket = setup_rocket_instance(app_config, rocket, false)
        .await
        .unwrap();
    let database_dropper = DatabaseDropper::new(
        &database_url_base,
        &maintenance_database_name,
        &database_name,
    );

    (rocket, database_dropper)
}

pub mod helpers {
    use crate::{
        db::models::{User, UserSession},
        services::{AuthService, UserService},
    };

    pub async fn create_user(id: &str, user_service: &UserService) -> User {
        let user = user_service
            .create_user(
                &format!("{}_user", id),
                &format!("{}_user@example.com", id),
                &format!("{}_user_pw", id),
            )
            .await
            .unwrap();
        user
    }

    pub async fn create_initial_user(
        auth_service: &AuthService,
        user_service: &UserService,
    ) -> (User, UserSession) {
        let user = create_user("initial", user_service).await;
        let user_session = auth_service.create_user_session(user.id).await.unwrap();
        (user, user_session)
    }
}
