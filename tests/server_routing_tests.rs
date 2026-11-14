use axum::{
    body::to_bytes,
    http::{Request, StatusCode},
};
use std::sync::Arc;
use taskflow::{
    config::Config,
    database::create_connection_pool,
    routes::{create_router, AppState},
};
use tower::ServiceExt;

#[tokio::test]
async fn test_health_check_endpoint() {
    // In-memory SQLite for testing
    let pool = create_connection_pool("sqlite::memory:")
        .await
        .expect("Failed to create in-memory db pool");

    let config = Arc::new(Config {
        server_host: "127.0.0.1".to_string(),
        server_port: 3000,
        database_url: "sqlite::memory:".to_string(),
        session_secret: "test-secret-at-least-32-chars-long-security-check".to_string(),
        session_max_age_secs: 3600,
        environment: "test".to_string(),
        admin_username: None,
        admin_email: None,
        admin_password: None,
    });

    let state = AppState {
        db: pool,
        config,
    };

    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify security headers
    assert_eq!(
        response.headers().get("x-content-type-options").unwrap(),
        "nosniff"
    );
    assert_eq!(
        response.headers().get("x-frame-options").unwrap(),
        "DENY"
    );

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "healthy");
    assert_eq!(json["service"], "TASKFLOW");
}

#[tokio::test]
async fn test_fallback_404() {
    let pool = create_connection_pool("sqlite::memory:")
        .await
        .expect("Failed to create in-memory db pool");

    let config = Arc::new(Config {
        server_host: "127.0.0.1".to_string(),
        server_port: 3000,
        database_url: "sqlite::memory:".to_string(),
        session_secret: "test-secret-at-least-32-chars-long-security-check".to_string(),
        session_max_age_secs: 3600,
        environment: "test".to_string(),
        admin_username: None,
        admin_email: None,
        admin_password: None,
    });

    let state = AppState {
        db: pool,
        config,
    };

    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/nonexistent-endpoint-xyz")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_static_asset_serving() {
    let pool = create_connection_pool("sqlite::memory:")
        .await
        .expect("Failed to create in-memory db pool");

    let config = Arc::new(Config {
        server_host: "127.0.0.1".to_string(),
        server_port: 3000,
        database_url: "sqlite::memory:".to_string(),
        session_secret: "test-secret-at-least-32-chars-long-security-check".to_string(),
        session_max_age_secs: 3600,
        environment: "test".to_string(),
        admin_username: None,
        admin_email: None,
        admin_password: None,
    });

    let state = AppState {
        db: pool,
        config,
    };

    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/static/css/style.css")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let css = String::from_utf8(body.to_vec()).unwrap();
    assert!(css.contains("--bg-primary"));
}

#[tokio::test]
async fn test_web_route_rendering() {
    let pool = create_connection_pool("sqlite::memory:")
        .await
        .expect("Failed to create in-memory db pool");

    let config = Arc::new(Config {
        server_host: "127.0.0.1".to_string(),
        server_port: 3000,
        database_url: "sqlite::memory:".to_string(),
        session_secret: "test-secret-at-least-32-chars-long-security-check".to_string(),
        session_max_age_secs: 3600,
        environment: "test".to_string(),
        admin_username: None,
        admin_email: None,
        admin_password: None,
    });

    let state = AppState {
        db: pool,
        config,
    };

    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("TASKFLOW"));
}
