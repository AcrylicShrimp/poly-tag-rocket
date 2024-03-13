use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CreatingStagingFile<'a> {
    pub name: &'a str,
    pub mime: Option<&'a str>,
}
