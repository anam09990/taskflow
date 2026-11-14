use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
#[allow(dead_code)]
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    Validation(String),
    InternalServerError(String),
    Database(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::Validation(msg) => write!(f, "Validation Error: {}", msg),
            AppError::InternalServerError(msg) => write!(f, "Internal Server Error: {}", msg),
            AppError::Database(msg) => write!(f, "Database Error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match &self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),
            AppError::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR", msg.clone()),
            AppError::InternalServerError(msg) => {
                tracing::error!("Internal server error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "An unexpected error occurred.".to_string())
            }
            AppError::Database(msg) => {
                tracing::error!("Database error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", "A database operation failed.".to_string())
            }
        };

        let body = Json(json!({
            "success": false,
            "error": {
                "type": error_type,
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("Requested resource was not found.".to_string()),
            sqlx::Error::Database(db_err) => {
                let msg = db_err.message();
                if msg.contains("UNIQUE constraint failed") {
                    AppError::Conflict("A record with this identifier already exists.".to_string())
                } else {
                    AppError::Database(msg.to_string())
                }
            }
            other => AppError::Database(other.to_string()),
        }
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        AppError::Validation(err.to_string())
    }
}
