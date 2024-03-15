mod config;
mod db;
mod dto;
mod fairings;
mod guards;
mod logger;
mod routes;
mod services;

#[cfg(test)]
mod test;

use crate::{
    config::AppConfig,
    fairings::staging_file_remover::StagingFileRemover,
    services::{local_file_system::LocalFileSystem, StagingFileService},
};
use chrono::Duration;
use clap::{Arg, ArgAction, Command, ValueHint};
use const_format::formatcp;
use dto::Error;
use rocket::{catch, catchers, http::Status, serde::json::Json, Build, Request, Rocket};
use std::{path::Path, sync::Arc};
use thiserror::Error;

fn cli() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .version(formatcp!(
            "{} ({} {})",
            env!("CARGO_PKG_VERSION"),
            env!("COMMIT_HASH"),
            env!("COMMIT_DATE")
        ))
        .args_conflicts_with_subcommands(true)
        .arg(
            Arg::new("config")
                .help("Path to the config file")
                .short('c')
                .long("config")
                .value_name("PATH")
                .value_hint(ValueHint::FilePath)
                .required(false)
                .allow_hyphen_values(true)
                .num_args(1),
        )
        .subcommand(
            Command::new("generate-config")
                .about("Generate a new config file")
                .long_about("Generate a new config file with the default values.")
                .arg(
                    Arg::new("config")
                        .help("Path to the config file")
                        .short('c')
                        .long("config")
                        .value_name("PATH")
                        .value_hint(ValueHint::FilePath)
                        .required(true)
                        .allow_hyphen_values(true)
                        .num_args(1),
                )
                .arg(
                    Arg::new("overwrite")
                        .help("Overwrite the file if it already exists")
                        .long("overwrite")
                        .action(ArgAction::SetTrue)
                ),
        )
        .subcommand(
            Command::new("test-config")
                .about("Print the config")
                .long_about("Print the config from the given file. This is useful for testing the config file.")
                .arg(
                    Arg::new("config")
                        .help("Path to the config file")
                        .short('c')
                        .long("config")
                        .value_name("PATH")
                        .value_hint(ValueHint::FilePath)
                        .required(false)
                        .allow_hyphen_values(true)
                        .num_args(1),
                ),
        )
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    IOError(#[from] std::io::Error),
    #[error("{0}")]
    DBError(#[from] db::DBError),
    #[error("{0}")]
    RocketError(#[from] rocket::Error),
    #[error("{0}")]
    FigmentError(#[from] figment::Error),
}

#[rocket::main]
async fn main() {
    let cli_matches = cli().get_matches();

    let result = match cli_matches.subcommand() {
        Some(("generate-config", sub_matches)) => {
            let config_path = sub_matches.get_one::<String>("config").unwrap();
            let overwrite = sub_matches.get_flag("overwrite");
            generate_config(config_path, overwrite)
        }
        Some(("test-config", sub_matches)) => {
            let config_path = sub_matches.get_one::<String>("config");
            test_config(config_path)
        }
        _ => {
            let config_path = cli_matches.get_one::<String>("config");
            run_server(config_path).await
        }
    };

    // Humanize the message if it's an error.
    if let Err(err) = result {
        let mut err = err.to_string();

        if let Some(first) = err.chars().next() {
            if first.is_ascii_lowercase() {
                err = first.to_uppercase().to_string() + &err[1..];
            }
        }

        if let Some(last) = err.chars().last() {
            match last {
                '.' | '!' | '?' => {}
                _ => err.push('.'),
            }
        }

        eprintln!("Command failed.");
        eprintln!("{}", err);
    }
}

fn generate_config(config_path: impl AsRef<Path>, overwrite: bool) -> Result<(), AppError> {
    let config_path = config_path.as_ref();

    if config_path.exists() {
        if !overwrite {
            eprintln!("The file already exists. Use the `--overwrite` flag to overwrite it.");
            eprintln!("Configuration is not generated.");
            return Ok(());
        }

        println!("The file already exists. Overwriting it.");
    }

    const JSON_CONFIG: &str = include_str!("./config/default.json");
    const TOML_CONFIG: &str = include_str!("./config/default.toml");
    const YAML_CONFIG: &str = include_str!("./config/default.yaml");

    let (file_type, file_content) = match config_path.extension() {
        Some(ext) if ext.eq_ignore_ascii_case("json") => ("JSON", JSON_CONFIG),
        Some(ext) if ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml") => {
            ("YAML", YAML_CONFIG)
        }
        _ => ("TOML", TOML_CONFIG),
    };

    std::fs::write(config_path, file_content)?;

    let full_config_path = config_path.canonicalize()?;
    println!(
        "{} configuration has been generated at `{}`.",
        file_type,
        full_config_path.display()
    );

    Ok(())
}

fn test_config(config_path: Option<impl AsRef<Path> + Clone>) -> Result<(), AppError> {
    let app_config = AppConfig::load(config_path.clone())?;
    let rocket_config = app_config.make_rocket_config();

    if let Some(config_path) = &config_path {
        let config_path = config_path.as_ref().canonicalize()?;
        println!(
            "Configuration path has been set: `{}`",
            config_path.display()
        );
    }

    println!("Configuration has been loaded successfully.");

    println!("[Loaded Configuration]");
    println!("- address: {}", rocket_config.address);
    println!("- port: {}", rocket_config.port);
    println!("- file_base_path: {}", app_config.file_base_path.display());
    println!("- temp_base_path: {}", app_config.temp_base_path.display());
    println!("- database_url_base: {}", app_config.database_url_base);
    println!("- database_name: {}", app_config.database_name);

    println!("- limits:");
    println!("    - form: {}", rocket_config.limits.get("form").unwrap());
    println!(
        "    - data_form: {}",
        rocket_config.limits.get("data-form").unwrap()
    );
    println!("    - file: {}", rocket_config.limits.get("file").unwrap());
    println!(
        "    - string: {}",
        rocket_config.limits.get("string").unwrap()
    );
    println!(
        "    - bytes: {}",
        rocket_config.limits.get("bytes").unwrap()
    );
    println!("    - json: {}", rocket_config.limits.get("json").unwrap());
    println!(
        "    - msgpack: {}",
        rocket_config.limits.get("msgpack").unwrap()
    );
    println!(
        "- expired_staging_file_removal_period: {}",
        app_config.expired_staging_file_removal_period
    );
    println!(
        "- expired_staging_file_expiration: {}",
        app_config.expired_staging_file_expiration
    );

    Ok(())
}

async fn run_server(config_path: Option<impl AsRef<Path> + Clone>) -> Result<(), AppError> {
    logger::setup_logger();

    let app_config = AppConfig::load(config_path.clone())?;
    let rocket = create_rocket_instance(&app_config)?;

    if let Some(config_path) = &config_path {
        let config_path = config_path.as_ref().canonicalize()?;
        let config_path = config_path.display().to_string();
        log::info!(target: "init", config_path; "Configuration path has been set.");
    }

    log::info!(target: "init", app_config:serde; "Configuration has been loaded.");

    let rocket = setup_rocket_instance(app_config, rocket, true).await?;
    let _rocket = rocket.launch().await?;

    Ok(())
}

/// Creates a new Rocket instance from the given configuration.
pub fn create_rocket_instance(app_config: &AppConfig) -> Result<Rocket<Build>, AppError> {
    let rocket_config = app_config.make_rocket_config();
    let rocket = Rocket::custom(rocket_config);
    Ok(rocket)
}

/// Sets up the Rocket instance with the given configuration.
/// This function will run the database migrations and create the database connection pool
/// before registering the services and routes.
pub async fn setup_rocket_instance(
    app_config: AppConfig,
    rocket: Rocket<Build>,
    attach_fairings: bool,
) -> Result<Rocket<Build>, AppError> {
    let database_url_base = &app_config.database_url_base;
    let database_name = &app_config.database_name;

    log::info!(target: "db", database_url_base, database_name; "Running database migrations.");
    db::run_migrations(database_url_base, database_name)?;

    log::info!(target: "db", database_url_base, database_name; "Creating database connection pool.");
    let db_pool = db::create_database_connection_pool(database_url_base, database_name);
    let db_pool = match db_pool {
        Ok(db_pool) => db_pool,
        Err(err) => {
            log::error!(target: "db", database_url_base, database_name, err:err; "Failed to create database connection pool.");
            return Err(err.into());
        }
    };

    let temp_base_path = &app_config.temp_base_path;
    let file_base_path = &app_config.file_base_path;

    log::info!(target: "file_driver", temp_base_path:?, file_base_path:?; "Creating file driver.");
    let file_driver = LocalFileSystem::new(temp_base_path, file_base_path).await?;

    let rocket = rocket.register("/", catchers![default_catcher]);
    let rocket = services::register_services(rocket, db_pool, Arc::new(file_driver));
    let rocket = routes::register_routes(rocket);

    let rocket = if attach_fairings {
        let staging_file_remover = StagingFileRemover::new(
            Duration::new(app_config.expired_staging_file_removal_period as i64, 0).unwrap(),
            Duration::new(app_config.expired_staging_file_expiration as i64, 0).unwrap(),
            rocket.state::<Arc<StagingFileService>>().unwrap().clone(),
        );

        let rocket = rocket.attach(staging_file_remover);

        rocket
    } else {
        rocket
    };

    let rocket = rocket.manage(app_config);

    Ok(rocket)
}

#[catch(default)]
fn default_catcher(status: Status, _request: &Request) -> Json<Error> {
    Json(Error {
        error: status.reason_lossy().to_ascii_lowercase(),
    })
}
