use tonic::{Request, Response, Status};
use std::sync::Arc;

use crate::application::{AuthService, BlogService as AppBlogService};
use crate::infrastructure::jwt::JwtService;
use crate::domain::post::UpdatePostRequest as DomainUpdateReq;

use crate::proto::blog_service_server::BlogService;
use crate::proto::{
    RegisterRequest, LoginRequest, AuthResponse, User as ProtoUser,
    CreatePostRequest, PostResponse, Post as ProtoPost,
    GetPostRequest, UpdatePostRequest, DeletePostRequest, EmptyResponse,
    ListPostsRequest, ListPostsResponse,
};

pub struct BlogGrpcService {
    auth_service: Arc<AuthService>,
    blog_service: Arc<AppBlogService>,
    jwt_service: Arc<JwtService>,
}

impl BlogGrpcService {
    pub fn new(auth: Arc<AuthService>, blog: Arc<AppBlogService>, jwt: Arc<JwtService>) -> Self {
        Self { auth_service: auth, blog_service: blog, jwt_service: jwt }
    }

    fn get_user_id_from_request<T>(&self, req: &Request<T>) -> Result<i64, Status> {
        let metadata = req.metadata();
        let auth_header = metadata.get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("Missing or invalid Authorization header"))?;
        
        let claims = self.jwt_service.verify_token(auth_header)
            .map_err(|_| Status::unauthenticated("Invalid token"))?;
        
        Ok(claims.user_id)
    }

    fn to_proto_user(user: &crate::domain::user::User) -> ProtoUser {
        ProtoUser { id: user.id, username: user.username.clone(), email: user.email.clone() }
    }

    fn to_proto_post(post: &crate::domain::post::Post) -> ProtoPost {
        ProtoPost {
            id: post.id,
            title: post.title.clone(),
            content: post.content.clone(),
            author_id: post.author_id,
            author_name: post.author_name.clone(),
            created_at: post.created_at.to_rfc3339(),
        }
    }
}

#[tonic::async_trait]
impl BlogService for BlogGrpcService {
    async fn register(&self, request: Request<RegisterRequest>) -> Result<Response<AuthResponse>, Status> {
        let req = request.into_inner();
        let (token, user) = self.auth_service.register(crate::domain::user::RegisterRequest {
            username: req.username, email: req.email, password: req.password,
        }).await.map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(AuthResponse { token, user: Some(Self::to_proto_user(&user)) }))
    }

    async fn login(&self, request: Request<LoginRequest>) -> Result<Response<AuthResponse>, Status> {
        let req = request.into_inner();
        let (token, user) = self.auth_service.login(crate::domain::user::LoginRequest {
            username: req.username, password: req.password,
        }).await.map_err(|_| Status::unauthenticated("Invalid credentials"))?;
        Ok(Response::new(AuthResponse { token, user: Some(Self::to_proto_user(&user)) }))
    }

    async fn create_post(&self, request: Request<CreatePostRequest>) -> Result<Response<PostResponse>, Status> {
        let user_id = self.get_user_id_from_request(&request)?;
        let req = request.into_inner();
        let post = self.blog_service.create_post(user_id, crate::domain::post::CreatePostRequest {
            title: req.title, content: req.content,
        }).await.map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(PostResponse { post: Some(Self::to_proto_post(&post)) }))
    }

    async fn get_post(&self, request: Request<GetPostRequest>) -> Result<Response<PostResponse>, Status> {
        let req = request.into_inner();
        let post = self.blog_service.get_post(req.id).await.map_err(|e| match e {
            crate::domain::error::AppError::PostNotFound => Status::not_found("Post not found"),
            _ => Status::internal(e.to_string()),
        })?;
        Ok(Response::new(PostResponse { post: Some(Self::to_proto_post(&post)) }))
    }

    async fn update_post(&self, request: Request<UpdatePostRequest>) -> Result<Response<PostResponse>, Status> {
        let user_id = self.get_user_id_from_request(&request)?;
        let req = request.into_inner();
        let post = self.blog_service.update_post(req.id, user_id, DomainUpdateReq {
            title: if req.title.is_empty() { None } else { Some(req.title) },
            content: if req.content.is_empty() { None } else { Some(req.content) },
        }).await.map_err(|e| match e {
            crate::domain::error::AppError::PostNotFound => Status::not_found("Post not found"),
            crate::domain::error::AppError::Forbidden => Status::permission_denied("Not the author"),
            _ => Status::internal(e.to_string()),
        })?;
        Ok(Response::new(PostResponse { post: Some(Self::to_proto_post(&post)) }))
    }

    async fn delete_post(&self, request: Request<DeletePostRequest>) -> Result<Response<EmptyResponse>, Status> {
        let user_id = self.get_user_id_from_request(&request)?;
        let req = request.into_inner();
        self.blog_service.delete_post(req.id, user_id).await.map_err(|e| match e {
            crate::domain::error::AppError::PostNotFound => Status::not_found("Post not found"),
            crate::domain::error::AppError::Forbidden => Status::permission_denied("Not the author"),
            _ => Status::internal(e.to_string()),
        })?;
        Ok(Response::new(EmptyResponse {}))
    }

    async fn list_posts(&self, request: Request<ListPostsRequest>) -> Result<Response<ListPostsResponse>, Status> {
        let req = request.into_inner();
        let (posts, total) = self.blog_service.list_posts(req.limit, req.offset).await.map_err(|e| Status::internal(e.to_string()))?;
        
        let proto_posts = posts.iter().map(Self::to_proto_post).collect();
        Ok(Response::new(ListPostsResponse {
            posts: proto_posts,
            total: total as i32,
            limit: req.limit,
            offset: req.offset,
        }))
    }
}