use rocket::{routes, Build, Rocket};

pub fn register_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount("/tags", routes![])
}
