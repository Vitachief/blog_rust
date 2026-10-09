use crate::error::BlogClientError;
use crate::proto::blog_service_client::BlogServiceClient;
use crate::proto::{
    CreatePostRequest, DeletePostRequest, GetPostRequest, ListPostsRequest, LoginRequest,
    RegisterRequest, UpdatePostRequest,
};
use crate::{AuthResponse, ListPostsResponse, Post, User};
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;

pub struct GrpcClient {
    client: BlogServiceClient<Channel>,
    token: Option<String>,
}

impl GrpcClient {
    pub async fn new(url: String) -> Result<Self, BlogClientError> {
        let client = BlogServiceClient::connect(url).await?;
        Ok(Self { client, token: None })
    }

    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    fn attach_auth<T>(&self, mut request: Request<T>) -> Result<Request<T>, BlogClientError> {
        if let Some(token) = &self.token {
            let value: MetadataValue<_> = format!("Bearer {}", token)
                .parse()
                .map_err(|_| BlogClientError::InvalidRequest("Invalid token format".into()))?;
            request.metadata_mut().insert("authorization", value);
        }
        Ok(request)
    }

    pub async fn register(&mut self, username: String, email: String, password: String) -> Result<AuthResponse, BlogClientError> {
        let req = Request::new(RegisterRequest { username, email, password });
        let response = self.client.register(req).await?.into_inner();
        self.to_auth_response(response)
    }

    pub async fn login(&mut self, username: String, password: String) -> Result<AuthResponse, BlogClientError> {
        let req = Request::new(LoginRequest { username, password });
        let response = self.client.login(req).await?.into_inner();
        self.to_auth_response(response)
    }

    fn to_auth_response(&self, resp: crate::proto::AuthResponse) -> Result<AuthResponse, BlogClientError> {
        let proto_user = resp.user.ok_or_else(|| BlogClientError::ServerError("Missing user in response".into()))?;
        Ok(AuthResponse {
            token: resp.token,
            user: User { id: proto_user.id, username: proto_user.username, email: proto_user.email },
        })
    }

    fn to_post(proto_post: crate::proto::Post) -> Post {
        Post {
            id: proto_post.id,
            title: proto_post.title,
            content: proto_post.content,
            author_id: proto_post.author_id,
            created_at: proto_post.created_at,
        }
    }

    pub async fn create_post(&self, title: String, content: String) -> Result<Post, BlogClientError> {
        let req = Request::new(CreatePostRequest { title, content });
        let req = self.attach_auth(req)?;
        
        let mut client = self.client.clone();
        let response: tonic::Response<crate::proto::PostResponse> = client.create_post(req).await?;
        
        let inner = response.into_inner();
        inner.post.map(Self::to_post).ok_or_else(|| BlogClientError::ServerError("Missing post in response".into()))
    }

    pub async fn get_post(&self, id: i64) -> Result<Post, BlogClientError> {
        let req = Request::new(GetPostRequest { id });
        
        let mut client = self.client.clone();
        let response: tonic::Response<crate::proto::PostResponse> = client.get_post(req).await.map_err(|status| {
            if status.code() == tonic::Code::NotFound { BlogClientError::NotFound("Post not found".into()) } 
            else { BlogClientError::Grpc(status) }
        })?;
        
        let inner = response.into_inner();
        inner.post.map(Self::to_post).ok_or_else(|| BlogClientError::ServerError("Missing post in response".into()))
    }

    pub async fn update_post(&self, id: i64, title: Option<String>, content: Option<String>) -> Result<Post, BlogClientError> {
        let req = Request::new(UpdatePostRequest { id, title: title.unwrap_or_default(), content: content.unwrap_or_default() });
        let req = self.attach_auth(req)?;
        
        let mut client = self.client.clone();
        let response: tonic::Response<crate::proto::PostResponse> = client.update_post(req).await.map_err(|status| {
            match status.code() {
                tonic::Code::NotFound => BlogClientError::NotFound("Post not found".into()),
                tonic::Code::PermissionDenied => BlogClientError::Unauthorized("Not the author".into()),
                _ => BlogClientError::Grpc(status),
            }
        })?;
        
        let inner = response.into_inner();
        inner.post.map(Self::to_post).ok_or_else(|| BlogClientError::ServerError("Missing post in response".into()))
    }

    pub async fn delete_post(&self, id: i64) -> Result<(), BlogClientError> {
        let req = Request::new(DeletePostRequest { id });
        let req = self.attach_auth(req)?;
        
        let mut client = self.client.clone();
        client.delete_post(req).await.map_err(|status| {
            match status.code() {
                tonic::Code::NotFound => BlogClientError::NotFound("Post not found".into()),
                tonic::Code::PermissionDenied => BlogClientError::Unauthorized("Not the author".into()),
                _ => BlogClientError::Grpc(status),
            }
        })?;
        
        Ok(())
    }

    pub async fn list_posts(&self, limit: i32, offset: i32) -> Result<ListPostsResponse, BlogClientError> {
        let req = Request::new(ListPostsRequest { limit, offset });
        
        let mut client = self.client.clone();
        let response = client.list_posts(req).await?.into_inner();
        
        Ok(ListPostsResponse {
            posts: response.posts.into_iter().map(Self::to_post).collect(),
            total: response.total as i64,
            limit: response.limit,
            offset: response.offset,
        })
    }
}