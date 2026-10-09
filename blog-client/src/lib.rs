pub mod error;
pub mod http_client;
pub mod grpc_client;

// Реэкспорт сгенерированных proto-типов для использования снаружи
pub mod proto {
    tonic::include_proto!("blog");
}

use error::BlogClientError;
use serde::{Deserialize, Serialize};

/// Транспорт для взаимодействия с сервером
pub enum Transport {
    Http(String),
    Grpc(String),
}

/// Ответ аутентификации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

/// Пользователь
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
}

/// Пост
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    pub created_at: String,
}

/// Ответ списка постов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPostsResponse {
    pub posts: Vec<Post>,
    pub total: i64,
    pub limit: i32,
    pub offset: i32,
}

/// Основной клиент блога с единым интерфейсом
pub struct BlogClient {
    transport: Transport,
    http: Option<http_client::HttpClient>,
    grpc: Option<grpc_client::GrpcClient>,
    token: Option<String>,
}

impl BlogClient {
    pub async fn new(transport: Transport) -> Result<Self, BlogClientError> {
        let (http, grpc) = match &transport {
            Transport::Http(url) => {
                let client = http_client::HttpClient::new(url.clone())?;
                (Some(client), None)
            }
            Transport::Grpc(url) => {
                let client = grpc_client::GrpcClient::new(url.clone()).await?;
                (None, Some(client))
            }
        };

        Ok(Self {
            transport,
            http,
            grpc,
            token: None,
        })
    }

    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
        self.propagate_token();
    }

    pub fn get_token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    fn propagate_token(&mut self) {
        if let Some(token) = &self.token {
            if let Some(http) = &mut self.http {
                http.set_token(token.clone());
            }
            if let Some(grpc) = &mut self.grpc {
                grpc.set_token(token.clone());
            }
        }
    }

    fn require_auth(&self) -> Result<(), BlogClientError> {
        if self.token.is_none() {
            return Err(BlogClientError::AuthRequired);
        }
        Ok(())
    }

    // === Аутентификация ===

    pub async fn register(
        &mut self,
        username: String,
        email: String,
        password: String,
    ) -> Result<AuthResponse, BlogClientError> {
        let response = match &mut self.transport {
            Transport::Http(_) => {
                self.http.as_mut().unwrap().register(&username, &email, &password).await?
            }
            Transport::Grpc(_) => {
                self.grpc.as_mut().unwrap().register(username, email, password).await?
            }
        };
        self.set_token(response.token.clone());
        Ok(response)
    }

    pub async fn login(
        &mut self,
        username: String,
        password: String,
    ) -> Result<AuthResponse, BlogClientError> {
        let response = match &mut self.transport {
            Transport::Http(_) => {
                self.http.as_mut().unwrap().login(&username, &password).await?
            }
            Transport::Grpc(_) => {
                self.grpc.as_mut().unwrap().login(username, password).await?
            }
        };
        self.set_token(response.token.clone());
        Ok(response)
    }

    // === CRUD постов ===

    pub async fn create_post(
        &self,
        title: String,
        content: String,
    ) -> Result<Post, BlogClientError> {
        self.require_auth()?;
        match &self.transport {
            Transport::Http(_) => self.http.as_ref().unwrap().create_post(&title, &content).await,
            Transport::Grpc(_) => self.grpc.as_ref().unwrap().create_post(title, content).await,
        }
    }

    pub async fn get_post(&self, id: i64) -> Result<Post, BlogClientError> {
        match &self.transport {
            Transport::Http(_) => self.http.as_ref().unwrap().get_post(id).await,
            Transport::Grpc(_) => self.grpc.as_ref().unwrap().get_post(id).await,
        }
    }

    pub async fn update_post(
        &self,
        id: i64,
        title: Option<String>,
        content: Option<String>,
    ) -> Result<Post, BlogClientError> {
        self.require_auth()?;
        match &self.transport {
            Transport::Http(_) => self.http.as_ref().unwrap().update_post(id, title, content).await,
            Transport::Grpc(_) => self.grpc.as_ref().unwrap().update_post(id, title, content).await,
        }
    }

    pub async fn delete_post(&self, id: i64) -> Result<(), BlogClientError> {
        self.require_auth()?;
        match &self.transport {
            Transport::Http(_) => self.http.as_ref().unwrap().delete_post(id).await,
            Transport::Grpc(_) => self.grpc.as_ref().unwrap().delete_post(id).await,
        }
    }

    pub async fn list_posts(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<ListPostsResponse, BlogClientError> {
        match &self.transport {
            Transport::Http(_) => self.http.as_ref().unwrap().list_posts(limit, offset).await,
            Transport::Grpc(_) => self.grpc.as_ref().unwrap().list_posts(limit, offset).await,
        }
    }
}