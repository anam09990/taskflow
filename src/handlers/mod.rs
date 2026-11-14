pub mod admin_handler;
pub mod analytics_handler;
pub mod auth_handler;
pub mod calendar_handler;
pub mod project_handler;
pub mod task_handler;

use axum::{response::Json, http::StatusCode};
use serde_json::{json, Value};

pub async fn health_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "service": "TASKFLOW",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
}
