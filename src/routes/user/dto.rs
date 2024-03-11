use crate::db::models::User;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CreatingUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password: &'a str,
}

#[derive(Serialize, Deserialize)]
pub struct SettingUserUsername<'a> {
    pub username: &'a str,
}

#[derive(Serialize, Deserialize)]
pub struct SettingUserPassword<'a> {
    pub password: &'a str,
}

#[derive(Serialize, Deserialize)]
pub struct UserList {
    pub users: Vec<User>,
    pub last_user_id: Option<i32>,
    pub limit: u32,
}
