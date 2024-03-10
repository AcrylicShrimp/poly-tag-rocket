use crate::db::models::User;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CreatingUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct SettingUserUsername {
    pub username: String,
}

#[derive(Serialize, Deserialize)]
pub struct SettingUserPassword {
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct UserList {
    pub users: Vec<User>,
    pub last_user_id: Option<i32>,
    pub limit: u32,
}
