use argon2::{
    password_hash::{
        rand_core::{OsRng, RngCore},
        SaltString,
    },
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use base64::{prelude::BASE64_STANDARD, Engine};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PasswordServiceError {
    #[error("argon2 error: {0}")]
    Argon2Error(#[from] argon2::password_hash::Error),
}

pub struct PasswordService;

impl PasswordService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    pub fn argon2(&self) -> Argon2 {
        Argon2::default()
    }

    pub fn salt_string(&self) -> SaltString {
        SaltString::generate(&mut OsRng)
    }

    pub fn generate_secure_token_252(&self) -> String {
        let mut buf = [0u8; 189];
        OsRng.fill_bytes(&mut buf);
        BASE64_STANDARD.encode(buf)
    }

    pub fn hash_password(&self, password: &str) -> Result<String, PasswordServiceError> {
        let argon2 = self.argon2();
        let salt = self.salt_string();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)?
            .to_string();
        Ok(password_hash)
    }

    pub fn verify_password_hash(
        &self,
        password: &str,
        password_hash: &str,
    ) -> Result<bool, PasswordServiceError> {
        let argon2 = self.argon2();
        let password_hash = PasswordHash::new(password_hash)?;
        let matches = argon2
            .verify_password(password.as_bytes(), &password_hash)
            .is_ok();
        Ok(matches)
    }
}
