pub mod models;
pub mod schema;

use diesel::{Connection, PgConnection};
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use thiserror::Error;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/db/migrations");

#[derive(Error, Debug)]
pub enum DBError {
    #[error("failed to connect to database: {0}")]
    ConnectionError(#[from] diesel::ConnectionError),
    #[error("failed to run migrations: {0}")]
    MigrationError(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("failed to create database connection pool: {0}")]
    PoolCreationError(#[from] diesel_async::pooled_connection::deadpool::BuildError),
    #[error("failed to execute query: {0}")]
    DieselError(#[from] diesel::result::Error),
}

pub fn run_migrations(database_url_base: &str, database_name: &str) -> Result<(), DBError> {
    let url = make_database_url(database_url_base, database_name);
    let mut connection = PgConnection::establish(&url)?;
    connection.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}

pub fn create_database_connection_pool(
    database_url_base: &str,
    database_name: &str,
) -> Result<Pool<AsyncPgConnection>, DBError> {
    let url = make_database_url(database_url_base, database_name);
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(url);
    let pool = Pool::builder(manager).build()?;
    Ok(pool)
}

fn make_database_url(database_url_base: &str, database_name: &str) -> String {
    if database_url_base.ends_with('/') {
        format!("{}{}", database_url_base, database_name)
    } else {
        format!("{}/{}", database_url_base, database_name)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use diesel::RunQueryDsl;

    pub struct DatabaseDropper {
        database_url_base: String,
        maintenance_database_name: String,
        database_name: String,
    }

    impl DatabaseDropper {
        pub fn new(
            database_url_base: &str,
            maintenance_database_name: &str,
            database_name: &str,
        ) -> Self {
            Self {
                database_url_base: database_url_base.to_string(),
                maintenance_database_name: maintenance_database_name.to_string(),
                database_name: database_name.to_string(),
            }
        }
    }

    impl Drop for DatabaseDropper {
        fn drop(&mut self) {
            let url = make_database_url(&self.database_url_base, &self.maintenance_database_name);
            let mut connection = PgConnection::establish(&url).unwrap();
            let query = format!("DROP DATABASE \"{}\"", &self.database_name);
            diesel::sql_query(query).execute(&mut connection).unwrap();
        }
    }

    /// Creates a new database used for testing.
    /// Returns the name of the new database.
    pub fn create_test_database(
        database_url_base: &str,
        maintenance_database_name: &str,
        id: &str,
    ) -> Result<String, DBError> {
        let test_database_name = format!("__test_{}", id);

        let url = make_database_url(database_url_base, maintenance_database_name);
        let mut connection = PgConnection::establish(&url)?;
        let query = format!("CREATE DATABASE \"{}\"", test_database_name);
        diesel::sql_query(query).execute(&mut connection)?;

        Ok(test_database_name)
    }
}
