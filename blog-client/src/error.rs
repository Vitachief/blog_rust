use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlogClientError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("Transport connection error: {0}")]
    Transport(#[from] tonic::transport::Error),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Authentication required. Please login first.")]
    AuthRequired,

    #[error("Server error: {0}")]
    ServerError(String),
}