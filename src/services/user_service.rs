use super::{password_service, PasswordService};
use crate::db::models::{CreatingUser, User};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserServiceError {
    #[error("database pool error: {0}")]
    PoolError(#[from] diesel_async::pooled_connection::deadpool::PoolError),
    #[error("diesel error: {0}")]
    DieselError(#[from] diesel::result::Error),
    #[error("{0}")]
    PasswordServiceError(#[from] password_service::PasswordServiceError),
}

pub struct UserService {
    db_pool: Pool<AsyncPgConnection>,
    password_service: Arc<PasswordService>,
}

impl UserService {
    pub fn new(
        db_pool: Pool<AsyncPgConnection>,
        password_service: Arc<PasswordService>,
    ) -> Arc<Self> {
        Arc::new(Self {
            db_pool,
            password_service,
        })
    }

    /// Creates a new user. Their password will be hashed before being stored in the database.
    pub async fn create_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<User, UserServiceError> {
        use crate::db::schema;

        let password_hash = self.password_service.hash_password(password)?;

        let db = &mut self.db_pool.get().await?;
        let user = diesel::insert_into(schema::users::table)
            .values(CreatingUser {
                username,
                email,
                password: &password_hash,
            })
            .returning((
                schema::users::id,
                schema::users::username,
                schema::users::email,
                schema::users::joined_at,
            ))
            .get_result::<User>(db)
            .await?;

        Ok(user)
    }

    /// Removes a user by their ID.
    /// Returns the user that was removed, or `None` if the user was not found.
    pub async fn remove_user_by_id(&self, user_id: i32) -> Result<Option<User>, UserServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let deleted_user =
            diesel::delete(schema::users::dsl::users.filter(schema::users::id.eq(user_id)))
                .returning((
                    schema::users::id,
                    schema::users::username,
                    schema::users::email,
                    schema::users::joined_at,
                ))
                .get_result::<User>(db)
                .await
                .optional()?;

        Ok(deleted_user)
    }

    /// Retrieves a list of users.
    /// The result will be sorted by user ID in ascending order.
    /// If `last_user_id` is provided, the result will start after the user with that ID.
    pub async fn get_users(
        &self,
        last_user_id: Option<i32>,
        limit: u32,
    ) -> Result<Vec<User>, UserServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let users = schema::users::dsl::users
            .filter(schema::users::id.gt(last_user_id.unwrap_or(0)))
            .select((
                schema::users::id,
                schema::users::username,
                schema::users::email,
                schema::users::joined_at,
            ))
            .order(schema::users::id.asc())
            .limit(limit as i64)
            .load::<User>(db)
            .await?;

        Ok(users)
    }

    /// Retrieves a user by their ID.
    pub async fn get_user_by_id(&self, user_id: i32) -> Result<Option<User>, UserServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let user = schema::users::dsl::users
            .filter(schema::users::id.eq(user_id))
            .select((
                schema::users::id,
                schema::users::username,
                schema::users::email,
                schema::users::joined_at,
            ))
            .first::<User>(db)
            .await
            .optional()?;

        Ok(user)
    }

    /// Updates a user's email by their ID.
    /// Returns the updated user, or `None` if the user was not found.
    pub async fn set_user_username_by_id(
        &self,
        user_id: i32,
        new_username: &str,
    ) -> Result<Option<User>, UserServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let updated_user =
            diesel::update(schema::users::dsl::users.filter(schema::users::id.eq(user_id)))
                .set(schema::users::username.eq(new_username))
                .returning((
                    schema::users::id,
                    schema::users::username,
                    schema::users::email,
                    schema::users::joined_at,
                ))
                .get_result::<User>(db)
                .await
                .optional()?;

        Ok(updated_user)
    }

    /// Updates a user's password by their ID.
    /// The new password will be hashed before being stored in the database.
    /// Returns the updated user, or `None` if the user was not found.
    pub async fn set_user_password_by_id(
        &self,
        user_id: i32,
        new_password: &str,
    ) -> Result<Option<User>, UserServiceError> {
        use crate::db::schema;

        let password_hash = self.password_service.hash_password(new_password)?;

        let db = &mut self.db_pool.get().await?;
        let updated_user =
            diesel::update(schema::users::dsl::users.filter(schema::users::id.eq(user_id)))
                .set(schema::users::password.eq(&password_hash))
                .returning((
                    schema::users::id,
                    schema::users::username,
                    schema::users::email,
                    schema::users::joined_at,
                ))
                .get_result::<User>(db)
                .await
                .optional()?;

        Ok(updated_user)
    }
}
