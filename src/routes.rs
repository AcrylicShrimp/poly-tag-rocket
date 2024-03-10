pub mod collection;
pub mod file;
pub mod user;

use rocket::{Build, Rocket};

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    user::controllers::register_routes(rocket)
}
