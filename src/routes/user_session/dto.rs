use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CreatingUserSession<'a> {
    pub email: &'a str,
    pub password: &'a str,
}
