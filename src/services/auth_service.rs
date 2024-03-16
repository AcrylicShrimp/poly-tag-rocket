use super::{password_service, PasswordService};
use crate::db::models::{CreatingUserSession, User, UserIdWithPassword, UserSession};
use diesel::{BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::{pooled_connection::deadpool::Pool, AsyncPgConnection, RunQueryDsl};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthServiceError {
    #[error("database pool error: {0}")]
    Pool(#[from] diesel_async::pooled_connection::deadpool::PoolError),
    #[error("diesel error: {0}")]
    Diesel(#[from] diesel::result::Error),
    #[error("{0}")]
    PasswordService(#[from] password_service::PasswordServiceError),
}

pub struct AuthService {
    db_pool: Pool<AsyncPgConnection>,
    password_service: Arc<PasswordService>,
}

impl AuthService {
    pub fn new(
        db_pool: Pool<AsyncPgConnection>,
        password_service: Arc<PasswordService>,
    ) -> Arc<Self> {
        Arc::new(Self {
            db_pool,
            password_service,
        })
    }

    /// Authenticates a user by their email and password.
    /// Returns the user ID if the authentication is successful, otherwise None.
    pub async fn authenticate_user(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<i32>, AuthServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let user = schema::users::dsl::users
            .filter(schema::users::email.eq(email))
            .select((schema::users::id, schema::users::password))
            .first::<UserIdWithPassword>(db)
            .await
            .optional()?;

        let user = match user {
            Some(user) => user,
            None => {
                // prevent timing attacks by hashing a fake password
                self.password_service.hash_password(password)?;
                return Ok(None);
            }
        };

        if !self
            .password_service
            .verify_password_hash(password, &user.password)?
        {
            return Ok(None);
        }

        Ok(Some(user.id))
    }

    /// Creates a new user session for the given user ID.
    pub async fn create_user_session(&self, user_id: i32) -> Result<UserSession, AuthServiceError> {
        use crate::db::schema;

        let token = self.password_service.generate_secure_token_252();

        let db = &mut self.db_pool.get().await?;
        let user_session = diesel::insert_into(schema::user_sessions::table)
            .values(CreatingUserSession {
                user_id,
                token: &token,
            })
            .returning((
                schema::user_sessions::user_id,
                schema::user_sessions::token,
                schema::user_sessions::created_at,
            ))
            .get_result::<UserSession>(db)
            .await?;

        Ok(user_session)
    }

    /// Removes a user session from the database.
    /// Returns the user session that was removed, or `None` if the user session was not found.
    pub async fn remove_user_session(
        &self,
        user_id: i32,
        token: &str,
    ) -> Result<Option<UserSession>, AuthServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let deleted_user_session = diesel::delete(
            schema::user_sessions::dsl::user_sessions.filter(
                schema::user_sessions::token
                    .eq(token)
                    .and(schema::user_sessions::user_id.eq(user_id)),
            ),
        )
        .returning((
            schema::user_sessions::user_id,
            schema::user_sessions::token,
            schema::user_sessions::created_at,
        ))
        .get_result::<UserSession>(db)
        .await
        .optional()?;

        Ok(deleted_user_session)
    }

    /// Gets a user from by session token.
    /// Returns the user if the session is found, otherwise None.
    pub async fn get_user_from_session(
        &self,
        token: &str,
    ) -> Result<Option<User>, AuthServiceError> {
        use crate::db::schema;

        let db = &mut self.db_pool.get().await?;
        let user = schema::users::table
            .inner_join(schema::user_sessions::table)
            .filter(schema::user_sessions::token.eq(token))
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
}
