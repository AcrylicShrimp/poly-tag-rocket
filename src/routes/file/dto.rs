use crate::db::models::Collection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct CreatingFile<'a> {
    pub name: &'a str,
    pub mime: Option<&'a str>,
}
