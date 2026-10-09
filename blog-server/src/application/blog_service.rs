use std::sync::Arc;
use crate::domain::{error::AppError, post::{Post, CreatePostRequest, UpdatePostRequest}};
use crate::data::post_repository::PostRepository;

pub struct BlogService {
    post_repo: Arc<PostRepository>,
}

impl BlogService {
    pub fn new(post_repo: Arc<PostRepository>) -> Self {
        Self { post_repo }
    }

    pub async fn create_post(&self, author_id: i64, req: CreatePostRequest) -> Result<Post, AppError> {
        self.post_repo.create(author_id, &req).await
    }

    pub async fn get_post(&self, id: i64) -> Result<Post, AppError> {
        self.post_repo.get_by_id(id).await?.ok_or(AppError::PostNotFound)
    }

    pub async fn update_post(&self, id: i64, author_id: i64, req: UpdatePostRequest) -> Result<Post, AppError> {
        self.post_repo.update(id, author_id, &req).await?.ok_or(AppError::Forbidden)
    }

    pub async fn delete_post(&self, id: i64, author_id: i64) -> Result<(), AppError> {
        let deleted = self.post_repo.delete(id, author_id).await?;
        if !deleted {
            if self.post_repo.get_by_id(id).await?.is_some() {
                return Err(AppError::Forbidden);
            }
            return Err(AppError::PostNotFound);
        }
        Ok(())
    }

    pub async fn list_posts(&self, limit: i32, offset: i32) -> Result<(Vec<Post>, i64), AppError> {
        self.post_repo.list(limit, offset).await
    }
}