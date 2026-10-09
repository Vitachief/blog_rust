use sqlx::PgPool;
use crate::domain::{error::AppError, post::{Post, CreatePostRequest, UpdatePostRequest}};

pub struct PostRepository {
    pool: PgPool,
}

impl PostRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    pub async fn create(&self, author_id: i64, req: &CreatePostRequest) -> Result<Post, AppError> {
        let post = sqlx::query_as::<_, Post>(
            "INSERT INTO posts (title, content, author_id) VALUES ($1, $2, $3) RETURNING *"
        )
        .bind(&req.title)
        .bind(&req.content)
        .bind(author_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(post)
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<Post>, AppError> {
        let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(post)
    }

    pub async fn update(&self, id: i64, author_id: i64, req: &UpdatePostRequest) -> Result<Option<Post>, AppError> {
        let post = sqlx::query_as::<_, Post>(
            "UPDATE posts SET title = COALESCE($1, title), content = COALESCE($2, content), updated_at = NOW() 
             WHERE id = $3 AND author_id = $4 RETURNING *"
        )
        .bind(&req.title)
        .bind(&req.content)
        .bind(id)
        .bind(author_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(post)
    }

    pub async fn delete(&self, id: i64, author_id: i64) -> Result<bool, AppError> {
        let res = sqlx::query("DELETE FROM posts WHERE id = $1 AND author_id = $2")
            .bind(id)
            .bind(author_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn list(&self, limit: i32, offset: i32) -> Result<(Vec<Post>, i64), AppError> {
        let posts = sqlx::query_as::<_, Post>("SELECT * FROM posts ORDER BY created_at DESC LIMIT $1 OFFSET $2")
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts")
            .fetch_one(&self.pool)
            .await?;
            
        Ok((posts, total.0))
    }
}