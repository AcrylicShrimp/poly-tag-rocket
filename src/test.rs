use crate::{
    create_rocket_instance,
    db::{self, test::DatabaseDropper},
};
use rocket::{Build, Rocket};
use uuid::Uuid;

/// Creates a new Rocket instance for testing.
/// It creates a new database for the test and runs the migrations.
pub fn create_test_rocket_instance() -> (Rocket<Build>, DatabaseDropper) {
    let database_url_base =
        std::env::var("DATABASE_URL_BASE").expect("DATABASE_URL_BASE must be set");
    let maintenance_database_name =
        std::env::var("MAINTENANCE_DATABASE_NAME").expect("MAINTENANCE_DATABASE_NAME must be set");
    let id = Uuid::new_v4().to_string();

    let database_name =
        db::test::create_test_database(&database_url_base, &maintenance_database_name, &id)
            .unwrap();
    let rocket = create_rocket_instance(&database_name).unwrap();
    let database_dropper = DatabaseDropper::new(
        &database_url_base,
        &maintenance_database_name,
        &database_name,
    );

    (rocket, database_dropper)
}
