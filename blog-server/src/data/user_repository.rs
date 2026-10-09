use sqlx::PgPool;
use crate::domain::{error::AppError, user::{User, RegisterRequest}};
use crate::infrastructure::auth::hash_password;

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    pub async fn create(&self, req: &RegisterRequest) -> Result<User, AppError> {
        let password_hash = hash_password(&req.password)?;
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING *"
        )
        .bind(&req.username)
        .bind(&req.email)
        .bind(&password_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique constraint") {
                AppError::UserAlreadyExists
            } else {
                AppError::Database(e)
            }
        })?;
        Ok(user)
    }

    pub async fn get_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;
        Ok(user)
    }
}