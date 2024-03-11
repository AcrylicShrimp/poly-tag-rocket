pub mod collection;
pub mod file;
pub mod user;

use rocket::{Build, Rocket};

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    let rocket = collection::controllers::register_routes(rocket);
    let rocket = user::controllers::register_routes(rocket);
    rocket
}
