pub mod api;
pub mod web;

use axum::{
    middleware as axum_middleware,
    response::{IntoResponse, Json},
    routing::get,
    http::StatusCode,
    Router,
};
use serde_json::json;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use crate::{
    config::Config,
    database::DbPool,
    handlers::health_check,
    middleware::security_headers_middleware,
};

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub db: DbPool,
    pub config: Arc<Config>,
}

pub fn create_router(state: AppState) -> Router {
    let api_router = api::api_routes();
    let web_router = web::web_routes();

    let static_service = ServeDir::new("static");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .merge(web_router)
        .nest_service("/static", static_service)
        .route("/health", get(health_check))
        .nest("/api/v1", api_router)
        .fallback(fallback_404)
        .layer(axum_middleware::from_fn(security_headers_middleware))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn fallback_404() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "success": false,
            "error": {
                "type": "NOT_FOUND",
                "message": "The requested resource does not exist on this server."
            }
        })),
    )
}
