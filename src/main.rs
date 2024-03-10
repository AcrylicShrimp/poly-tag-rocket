mod db;
mod dto;
mod guards;
mod routes;
mod services;

#[cfg(test)]
mod test;

use dto::Error;
use log::info;
use rocket::{catch, catchers, http::Status, serde::json::Json, Build, Request, Rocket};
use std::time::SystemTime;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    DBError(#[from] db::DBError),
    #[error("{0}")]
    FernInitError(#[from] fern::InitError),
    #[error("{0}")]
    RocketError(#[from] rocket::Error),
}

/// Sets up the logger.
fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339_seconds(SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Warn)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}

/// Creates a new Rocket instance.
/// It runs the migrations and creates a database connection pool.
/// The result Rocket instance has all the services and routes registered.
pub fn create_rocket_instance(
    with_logger: bool,
    database_name: &str,
) -> Result<Rocket<Build>, AppError> {
    if with_logger {
        setup_logger()?;
    }

    let database_url_base =
        std::env::var("DATABASE_URL_BASE").expect("DATABASE_URL_BASE must be set");

    info!("running migrations");
    db::run_migrations(&database_url_base, database_name)?;

    info!("creating database connection pool");
    let db_pool = db::create_database_connection_pool(&database_url_base, database_name)
        .expect("failed to connect to database");

    info!("building rocket");
    let rocket = rocket::build().register("/", catchers![default_catcher]);
    let rocket = services::register_services(rocket, db_pool);
    let rocket = routes::register_routes(rocket);

    Ok(rocket)
}

#[rocket::main]
async fn main() -> Result<(), AppError> {
    let live_database_name =
        std::env::var("LIVE_DATABASE_NAME").expect("LIVE_DATABASE_NAME must be set");

    info!("creating rocket");
    let rocket = create_rocket_instance(true, &live_database_name)?;

    info!("launching rocket");
    let _rocket = rocket.launch().await?;

    Ok(())
}

#[catch(default)]
fn default_catcher(status: Status, _request: &Request) -> Json<Error> {
    Json(Error {
        error: status.reason_lossy().to_ascii_lowercase(),
    })
}
