use figment::{
    providers::{Env, Format, Json, Toml, YamlExtended},
    Figment,
};
use rocket::{
    config::Ident,
    data::{ByteUnit, Limits},
    Config,
};
use serde::{Deserialize, Serialize};
use std::{
    net::IpAddr,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct AppLimit {
    #[serde(default = "app_limit_defaults::form")]
    pub form: ByteUnit,
    #[serde(default = "app_limit_defaults::data_form")]
    pub data_form: ByteUnit,
    #[serde(default = "app_limit_defaults::file")]
    pub file: ByteUnit,
    #[serde(default = "app_limit_defaults::string")]
    pub string: ByteUnit,
    #[serde(default = "app_limit_defaults::bytes")]
    pub bytes: ByteUnit,
    #[serde(default = "app_limit_defaults::json")]
    pub json: ByteUnit,
    #[serde(default = "app_limit_defaults::msgpack")]
    pub msgpack: ByteUnit,
}

impl Default for AppLimit {
    fn default() -> Self {
        Self {
            form: Limits::FORM,
            data_form: Limits::DATA_FORM,
            file: Limits::FILE,
            string: Limits::STRING,
            bytes: Limits::BYTES,
            json: Limits::JSON,
            msgpack: Limits::MESSAGE_PACK,
        }
    }
}

mod app_limit_defaults {
    use rocket::data::{ByteUnit, Limits};

    pub fn form() -> ByteUnit {
        Limits::FORM
    }

    pub fn data_form() -> ByteUnit {
        Limits::DATA_FORM
    }

    pub fn file() -> ByteUnit {
        Limits::FILE
    }

    pub fn string() -> ByteUnit {
        Limits::STRING
    }

    pub fn bytes() -> ByteUnit {
        Limits::BYTES
    }

    pub fn json() -> ByteUnit {
        Limits::JSON
    }

    pub fn msgpack() -> ByteUnit {
        Limits::MESSAGE_PACK
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    /// The address to bind the server to.
    #[serde(default = "app_config_defaults::address")]
    pub address: IpAddr,
    /// The port to bind the server to.
    #[serde(default = "app_config_defaults::port")]
    pub port: u16,
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
    /// **DEVELOPMENT ENVIRONMENT ONLY**
    ///
    /// The name of the default or maintenance database in PostgreSQL.
    /// It is used to create databases during tests.
    #[cfg(test)]
    #[serde(default = "app_config_defaults::maintenance_database_name")]
    pub maintenance_database_name: String,
    /// The limits for the application.
    #[serde(default)]
    pub limits: AppLimit,
    /// The period to remove expired staging files.
    /// The period is in seconds.
    #[serde(default = "app_config_defaults::expired_staging_file_removal_period")]
    pub expired_staging_file_removal_period: u64,
    /// The expiration for staging files.
    /// The expiration is in seconds.
    #[serde(default = "app_config_defaults::expired_staging_file_expiration")]
    pub expired_staging_file_expiration: u64,
}

mod app_config_defaults {
    use std::net::IpAddr;

    pub fn address() -> IpAddr {
        IpAddr::from([127, 0, 0, 1])
    }

    pub fn port() -> u16 {
        8000
    }

    #[cfg(test)]
    pub fn maintenance_database_name() -> String {
        "postgres".to_owned()
    }

    pub fn expired_staging_file_removal_period() -> u64 {
        60 * 60
    }

    pub fn expired_staging_file_expiration() -> u64 {
        60 * 60 * 24
    }
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

        config.address = self.address;
        config.port = self.port;
        config.temp_dir = self.temp_base_path.clone().into();
        config.limits = self.make_limits();
        config.ident = Ident::none();
        config.keep_alive = 60;

        config
    }

    fn make_limits(&self) -> Limits {
        let mut limits = Limits::new();
        limits = limits.limit("form", self.limits.form);
        limits = limits.limit("data-form", self.limits.data_form);
        limits = limits.limit("file", self.limits.file);
        limits = limits.limit("string", self.limits.string);
        limits = limits.limit("bytes", self.limits.bytes);
        limits = limits.limit("json", self.limits.json);
        limits = limits.limit("msgpack", self.limits.msgpack);
        limits
    }
}
