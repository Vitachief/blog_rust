use crate::{AuthResponse, BlogClientError, ListPostsResponse, Post, User};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

pub struct HttpClient {
    base_url: String,
    client: Client,
    token: Option<String>,
}

impl HttpClient {
    pub fn new(base_url: String) -> Result<Self, BlogClientError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            base_url,
            client,
            token: None,
        })
    }

    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    fn auth_header(&self) -> Result<String, BlogClientError> {
        self.token
            .as_ref()
            .map(|t| format!("Bearer {}", t))
            .ok_or(BlogClientError::AuthRequired)
    }

    /// Универсальная обработка HTTP-ответов с преобразованием статусов в ошибки
    async fn handle_response<T: serde::de::DeserializeOwned>(
        response: reqwest::Response,
    ) -> Result<T, BlogClientError> {
        let status = response.status();
        let body = response.text().await?;

        if status.is_success() {
            serde_json::from_str(&body).map_err(|e| {
                BlogClientError::ServerError(format!("Failed to parse response: {}", e))
            })
        } else {
            match status.as_u16() {
                401 => Err(BlogClientError::Unauthorized(body)),
                403 => Err(BlogClientError::Unauthorized("Forbidden".into())),
                404 => Err(BlogClientError::NotFound(body)),
                409 => Err(BlogClientError::InvalidRequest("User already exists".into())),
                _ => Err(BlogClientError::ServerError(format!("HTTP {}: {}", status, body))),
            }
        }
    }

    // === Аутентификация ===

    pub async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        let payload = serde_json::json!({
            "username": username,
            "email": email,
            "password": password,
        });

        let response = self
            .client
            .post(format!("{}/api/auth/register", self.base_url))
            .json(&payload)
            .send()
            .await?;

        // Сервер возвращает {"token": "...", "user": {...}}
        let value: Value = Self::handle_response(response).await?;
        let token = value["token"].as_str().unwrap_or_default().to_string();
        let user: User = serde_json::from_value(value["user"].clone())
            .map_err(|e| BlogClientError::ServerError(e.to_string()))?;
        Ok(AuthResponse { token, user })
    }

    pub async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        let payload = serde_json::json!({
            "username": username,
            "password": password,
        });

        let response = self
            .client
            .post(format!("{}/api/auth/login", self.base_url))
            .json(&payload)
            .send()
            .await?;

        let value: Value = Self::handle_response(response).await?;
        let token = value["token"].as_str().unwrap_or_default().to_string();
        let user: User = serde_json::from_value(value["user"].clone())
            .map_err(|e| BlogClientError::ServerError(e.to_string()))?;
        Ok(AuthResponse { token, user })
    }

    pub async fn create_post(
        &self,
        title: &str,
        content: &str,
    ) -> Result<Post, BlogClientError> {
        let payload = serde_json::json!({
            "title": title,
            "content": content,
        });

        let response = self
            .client
            .post(format!("{}/api/posts", self.base_url))
            .header("Authorization", self.auth_header()?)
            .json(&payload)
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn get_post(&self, id: i64) -> Result<Post, BlogClientError> {
        let response = self
            .client
            .get(format!("{}/api/posts/{}", self.base_url, id))
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn update_post(
        &self,
        id: i64,
        title: Option<String>,
        content: Option<String>,
    ) -> Result<Post, BlogClientError> {
        let mut payload = serde_json::Map::new();
        if let Some(t) = title {
            payload.insert("title".into(), serde_json::Value::String(t));
        }
        if let Some(c) = content {
            payload.insert("content".into(), serde_json::Value::String(c));
        }

        let response = self
            .client
            .put(format!("{}/api/posts/{}", self.base_url, id))
            .header("Authorization", self.auth_header()?)
            .json(&payload)
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn delete_post(&self, id: i64) -> Result<(), BlogClientError> {
        let response = self
            .client
            .delete(format!("{}/api/posts/{}", self.base_url, id))
            .header("Authorization", self.auth_header()?)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() || status.as_u16() == 204 {
            Ok(())
        } else if status.as_u16() == 404 {
            Err(BlogClientError::NotFound("Post not found".into()))
        } else {
            Err(BlogClientError::ServerError(format!("HTTP {}", status)))
        }
    }

    pub async fn list_posts(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<ListPostsResponse, BlogClientError> {
        let response = self
            .client
            .get(format!("{}/api/posts", self.base_url))
            .query(&[("limit", limit), ("offset", offset)])
            .send()
            .await?;

        Self::handle_response(response).await
    }
}