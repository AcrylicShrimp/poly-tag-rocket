use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CreatingFile<'a> {
    pub name: &'a str,
    pub mime: Option<&'a str>,
}
