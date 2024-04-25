pub mod collection;
pub mod file;
pub mod staging_file;
pub mod tag;
pub mod user;
pub mod user_session;

use rocket::{Build, Rocket};

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    let rocket = collection::controllers::register_routes(rocket);
    let rocket = file::controllers::register_routes(rocket);
    let rocket = staging_file::controllers::register_routes(rocket);
    let rocket = tag::controllers::register_routes(rocket);
    let rocket = user::controllers::register_routes(rocket);
    let rocket = user_session::controllers::register_routes(rocket);
    rocket
}
