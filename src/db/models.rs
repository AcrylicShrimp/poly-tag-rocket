use chrono::NaiveDateTime;
use diesel::{
    associations::Identifiable, deserialize::Queryable, prelude::Insertable,
    query_builder::AsChangeset, Selectable,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Selectable, Queryable, Identifiable, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::collections)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Insertable, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::collections)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CreatingCollection<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
}

#[derive(Serialize, Deserialize, AsChangeset, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::collections)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpdatingCollection<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
}

#[derive(Serialize, Deserialize, Selectable, Queryable, Identifiable, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub joined_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Selectable, Queryable, Identifiable, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(rename_all = "camelCase")]
pub struct UserIdWithPassword {
    pub id: i32,
    pub password: String,
}

#[derive(Serialize, Deserialize, Insertable, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CreatingUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password: &'a str,
}

#[derive(Serialize, Deserialize, Selectable, Queryable, Identifiable, Debug, Clone, PartialEq)]
#[diesel(primary_key(user_id, token))]
#[diesel(table_name = crate::db::schema::user_sessions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(rename_all = "camelCase")]
pub struct UserSession {
    pub user_id: i32,
    pub token: String,
    pub created_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Insertable, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::user_sessions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CreatingUserSession<'a> {
    pub user_id: i32,
    pub token: &'a str,
}

#[derive(Serialize, Deserialize, Selectable, Queryable, Identifiable, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub id: Uuid,
    pub name: String,
    pub mime: String,
    pub size: i64,
    pub hash: i64,
    pub uploaded_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Insertable, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CreatingFile<'a> {
    pub id: Uuid,
    pub name: &'a str,
    pub mime: &'a str,
    pub size: i64,
    pub hash: i64,
}

#[derive(Serialize, Deserialize, Selectable, Queryable, Identifiable, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::collection_file_pairs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(collection_id, file_id))]
#[serde(rename_all = "camelCase")]
pub struct CollectionFilePair {
    pub collection_id: Uuid,
    pub file_id: Uuid,
}

#[derive(Serialize, Deserialize, Insertable, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::collection_file_pairs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CreatingCollectionFilePair {
    pub collection_id: Uuid,
    pub file_id: Uuid,
}

#[derive(Serialize, Deserialize, Selectable, Queryable, Identifiable, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::staging_files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(rename_all = "camelCase")]
pub struct StagingFile {
    pub id: Uuid,
    pub name: String,
    pub mime: Option<String>,
    pub size: i64,
    pub staged_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Insertable, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::staging_files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CreatingStagingFile<'a> {
    pub name: &'a str,
    pub mime: Option<&'a str>,
    pub size: i64,
}

#[derive(Serialize, Deserialize, AsChangeset, Debug, Clone, PartialEq)]
#[diesel(table_name = crate::db::schema::staging_files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpdatingStagingFile<'a> {
    pub name: &'a str,
    pub mime: Option<&'a str>,
}
