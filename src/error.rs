use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    BucketNotFound(String),
    BucketAlreadyExists(String),
    ObjectNotFound { bucket: String, key: String },
    InvalidBucketName(String),
    InvalidObjectKey(String),
    StorageError(String),
    IoError(std::io::Error),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    code: String,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::BucketNotFound(name) => (
                StatusCode::NOT_FOUND,
                "NoSuchBucket",
                format!("The specified bucket '{}' does not exist", name),
            ),
            AppError::BucketAlreadyExists(name) => (
                StatusCode::CONFLICT,
                "BucketAlreadyOwnedByYou",
                format!("The bucket '{}' already exists", name),
            ),
            AppError::ObjectNotFound { bucket, key } => (
                StatusCode::NOT_FOUND,
                "NoSuchKey",
                format!("The specified key '{}' does not exist in bucket '{}'", key, bucket),
            ),
            AppError::InvalidBucketName(reason) => (
                StatusCode::BAD_REQUEST,
                "InvalidBucketName",
                format!("Invalid bucket name: {}", reason),
            ),
            AppError::InvalidObjectKey(reason) => (
                StatusCode::BAD_REQUEST,
                "InvalidObjectKey",
                format!("Invalid object key: {}", reason),
            ),
            AppError::StorageError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "InternalError",
                format!("Storage error: {}", msg),
            ),
            AppError::IoError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "InternalError",
                format!("I/O error: {}", e),
            ),
        };

        let body = serde_json::to_string(&ErrorResponse {
            error: code.to_string(),
            code: code.to_string(),
            message,
        })
        .unwrap();

        (status, [("content-type", "application/json")], body).into_response()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::IoError(e)
    }
}
