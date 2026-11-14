pub mod api;
pub mod web;

use axum::{
    extract::Path,
    http::{header, HeaderValue, StatusCode},
    middleware as axum_middleware,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use rust_embed::RustEmbed;
use serde_json::json;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use crate::{
    config::Config,
    database::DbPool,
    handlers::health_check,
    middleware::security_headers_middleware,
};

#[derive(RustEmbed)]
#[folder = "static/"]
struct StaticAssets;

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub db: DbPool,
    pub config: Arc<Config>,
}

fn guess_mime(path: &str) -> &'static str {
    if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    }
}

async fn static_handler(Path(path): Path<String>) -> Response {
    let clean_path = path.trim_start_matches('/').replace('\\', "/");
    match StaticAssets::get(&clean_path) {
        Some(content) => {
            let mime = guess_mime(&clean_path);
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, HeaderValue::from_static(mime)),
                    (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=86400")),
                ],
                content.data,
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": {
                    "type": "NOT_FOUND",
                    "message": "Static asset not found."
                }
            })),
        )
            .into_response(),
    }
}

pub fn create_router(state: AppState) -> Router {
    let api_router = api::api_routes();
    let web_router = web::web_routes();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .merge(web_router)
        .route("/static/{*path}", get(static_handler))
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
