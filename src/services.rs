mod auth_service;
mod password_service;
mod user_service;

pub use auth_service::*;
pub use password_service::*;
pub use user_service::*;

use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection};
use rocket::{Build, Rocket};

pub fn register_services(rocket: Rocket<Build>, db_pool: Pool<AsyncPgConnection>) -> Rocket<Build> {
    let password_service = PasswordService::new();
    let auth_service = AuthService::new(db_pool.clone(), password_service.clone());
    let user_service = UserService::new(db_pool, password_service.clone());

    rocket
        .manage(password_service)
        .manage(auth_service)
        .manage(user_service)
}
