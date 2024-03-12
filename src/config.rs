use figment::{
    providers::{Env, Format, Json, Toml, YamlExtended},
    Figment,
};
use rocket::{
    config::Ident,
    data::{ByteUnit, Limits},
    Config,
};
use serde::Deserialize;
use std::{
    net::IpAddr,
    path::{Path, PathBuf},
};

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    /// The address to bind the server to.
    pub address: Option<IpAddr>,
    /// The port to bind the server to.
    pub port: Option<u16>,
    /// The base path for the file storage.
    pub file_base_path: PathBuf,
    /// The base path for temporary files.
    #[serde(default = "std::env::temp_dir")]
    pub temp_base_path: PathBuf,
    /// The base URL for the database, without the database name.
    /// The database must be a PostgreSQL database.
    /// e.g. `postgres://user:password@localhost:5432`
    pub database_url_base: String,
    /// The name of the database to use.
    /// The database must be exist and be empty.
    pub database_name: String,
    #[cfg(test)]
    /// **DEVELOPMENT ENVIRONMENT ONLY**
    ///
    /// The name of the default or maintenance database in PostgreSQL.
    /// It is used to create databases during tests.
    pub maintenance_database_name: Option<String>,
    /// The limits for the application.
    pub limits: Option<AppLimit>,
}

#[derive(Deserialize, Debug)]
pub struct AppLimit {
    pub form: Option<ByteUnit>,
    pub data_form: Option<ByteUnit>,
    pub file: Option<ByteUnit>,
    pub string: Option<ByteUnit>,
    pub bytes: Option<ByteUnit>,
    pub json: Option<ByteUnit>,
    pub msgpack: Option<ByteUnit>,
}

impl AppConfig {
    pub fn load(file_path: Option<impl AsRef<Path>>) -> Result<Self, figment::Error> {
        let mut figment = Figment::new().join(Env::raw());

        if let Some(file_path) = file_path {
            let file_path = file_path.as_ref();

            if !file_path.exists() {
                return Err(
                    format!("The given path `{}` is not exist.", file_path.display()).into(),
                );
            }

            match file_path.extension() {
                Some(ext) if ext.eq_ignore_ascii_case("json") => {
                    figment = figment.join(Json::file(file_path));
                }
                Some(ext)
                    if ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml") =>
                {
                    figment = figment.join(YamlExtended::file(file_path));
                }
                _ => {
                    figment = figment.join(Toml::file(file_path));
                }
            }
        }

        figment.extract()
    }

    pub fn make_rocket_config(&self) -> Config {
        let mut config = Config::default();

        if let Some(address) = self.address {
            config.address = address;
        }

        if let Some(port) = self.port {
            config.port = port;
        }

        config.temp_dir = self.temp_base_path.clone().into();

        let mut limits = Limits::default();

        if let Some(app_limits) = &self.limits {
            if let Some(form) = app_limits.form {
                limits = limits.limit("form", form);
            }
            if let Some(data_form) = app_limits.data_form {
                limits = limits.limit("data-form", data_form);
            }
            if let Some(file) = app_limits.file {
                limits = limits.limit("file", file);
            }
            if let Some(string) = app_limits.string {
                limits = limits.limit("string", string);
            }
            if let Some(bytes) = app_limits.bytes {
                limits = limits.limit("bytes", bytes);
            }
            if let Some(json) = app_limits.json {
                limits = limits.limit("json", json);
            }
            if let Some(msgpack) = app_limits.msgpack {
                limits = limits.limit("msgpack", msgpack);
            }
        }

        config.limits = limits;
        config.ident = Ident::none();
        config.keep_alive = 60;

        config
    }
}
