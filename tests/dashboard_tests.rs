use axum::{
    body::{to_bytes, Body},
    http::{header::COOKIE, Request, StatusCode},
};
use serde_json::Value;
use std::sync::Arc;
use taskflow::{
    config::Config,
    database::{create_connection_pool, run_migrations},
    routes::{create_router, AppState},
};
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_app() -> (axum::Router, AppState, String, i64) {
    let pool = create_connection_pool("sqlite::memory:")
        .await
        .expect("Failed to create in-memory db pool");

    run_migrations(&pool).await.expect("Migrations failed");

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
        db: pool.clone(),
        config,
    };

    let app = create_router(state.clone());

    // Register user
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (uuid, username, email, password_hash, role) VALUES (?, ?, ?, ?, ?) RETURNING id"
    )
    .bind(Uuid::new_v4().to_string())
    .bind("dashuser")
    .bind("dash@example.com")
    .bind("hash")
    .bind("user")
    .fetch_one(&pool)
    .await
    .unwrap();

    let session_token = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sessions (token, user_id, expires_at) VALUES (?, ?, datetime('now', '+1 hour'))"
    )
    .bind(&session_token)
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    (app, state, session_token, user_id)
}

#[tokio::test]
async fn test_dashboard_overview_unauthorized() {
    let (app, _, _, _) = setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/analytics/overview")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_dashboard_overview_metrics() {
    let (app, state, token, user_id) = setup_app().await;

    // Seed tasks: 2 completed, 1 pending
    sqlx::query(
        "INSERT INTO tasks (uuid, user_id, title, priority, status) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind("Task 1")
    .bind("high")
    .bind("done")
    .execute(&state.db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO tasks (uuid, user_id, title, priority, status) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind("Task 2")
    .bind("medium")
    .bind("done")
    .execute(&state.db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO tasks (uuid, user_id, title, priority, status, due_date) VALUES (?, ?, ?, ?, ?, datetime('now', '+2 days'))"
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind("Task 3 with deadline")
    .bind("urgent")
    .bind("in_progress")
    .execute(&state.db)
    .await
    .unwrap();

    // Query overview
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/analytics/overview")
                .header(COOKIE, format!("taskflow_session={}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["total_tasks"], 3);
    assert_eq!(json["data"]["completed_tasks"], 2);
    assert_eq!(json["data"]["pending_tasks"], 1);
    assert_eq!(json["data"]["completion_rate"], 67.0);

    // Verify upcoming deadline is listed
    let upcoming = json["data"]["upcoming_deadlines"].as_array().unwrap();
    assert_eq!(upcoming.len(), 1);
    assert_eq!(upcoming[0]["title"], "Task 3 with deadline");
    assert_eq!(upcoming[0]["priority"], "urgent");
}
