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

async fn setup_app_for_calendar() -> (axum::Router, AppState, String, String) {
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
    });

    let state = AppState {
        db: pool.clone(),
        config,
    };

    let app = create_router(state.clone());

    // User 1
    let user1_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (uuid, username, email, password_hash, role) VALUES (?, ?, ?, ?, ?) RETURNING id"
    )
    .bind(Uuid::new_v4().to_string())
    .bind("cal_user1")
    .bind("cal1@example.com")
    .bind("hash")
    .bind("user")
    .fetch_one(&pool)
    .await
    .unwrap();

    let token1 = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sessions (token, user_id, expires_at) VALUES (?, ?, datetime('now', '+1 hour'))")
        .bind(&token1)
        .bind(user1_id)
        .execute(&pool)
        .await
        .unwrap();

    // User 2
    let user2_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (uuid, username, email, password_hash, role) VALUES (?, ?, ?, ?, ?) RETURNING id"
    )
    .bind(Uuid::new_v4().to_string())
    .bind("cal_user2")
    .bind("cal2@example.com")
    .bind("hash")
    .bind("user")
    .fetch_one(&pool)
    .await
    .unwrap();

    let token2 = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sessions (token, user_id, expires_at) VALUES (?, ?, datetime('now', '+1 hour'))")
        .bind(&token2)
        .bind(user2_id)
        .execute(&pool)
        .await
        .unwrap();

    // Seed tasks for User 1:
    // - In Nov 2026:
    sqlx::query("INSERT INTO tasks (uuid, user_id, title, priority, status, due_date) VALUES (?, ?, ?, ?, ?, '2026-11-15 14:00:00')")
        .bind(Uuid::new_v4().to_string())
        .bind(user1_id)
        .bind("Nov 15 Task")
        .bind("urgent")
        .bind("todo")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO tasks (uuid, user_id, title, priority, status, due_date) VALUES (?, ?, ?, ?, ?, '2026-11-28 09:00:00')")
        .bind(Uuid::new_v4().to_string())
        .bind(user1_id)
        .bind("Nov 28 Task")
        .bind("medium")
        .bind("todo")
        .execute(&pool)
        .await
        .unwrap();

    // - In Dec 2026 (outside Nov):
    sqlx::query("INSERT INTO tasks (uuid, user_id, title, priority, status, due_date) VALUES (?, ?, ?, ?, ?, '2026-12-05 10:00:00')")
        .bind(Uuid::new_v4().to_string())
        .bind(user1_id)
        .bind("Dec 05 Task")
        .bind("low")
        .bind("todo")
        .execute(&pool)
        .await
        .unwrap();

    // - In Oct 2026 (outside Nov):
    sqlx::query("INSERT INTO tasks (uuid, user_id, title, priority, status, due_date) VALUES (?, ?, ?, ?, ?, '2026-10-31 23:59:59')")
        .bind(Uuid::new_v4().to_string())
        .bind(user1_id)
        .bind("Oct 31 Task")
        .bind("high")
        .bind("todo")
        .execute(&pool)
        .await
        .unwrap();

    (app, state, token1, token2)
}

#[tokio::test]
async fn test_calendar_month_filtering() {
    let (app, _, token1, _) = setup_app_for_calendar().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/calendar/events?year=2026&month=11")
                .header(COOKIE, format!("taskflow_session={}", token1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    let events = json["data"].as_array().unwrap();
    assert_eq!(events.len(), 2, "Only 2 tasks fall in November 2026");
    assert_eq!(events[0]["title"], "Nov 15 Task");
    assert_eq!(events[1]["title"], "Nov 28 Task");
}

#[tokio::test]
async fn test_calendar_cross_user_isolation() {
    let (app, _, _, token2) = setup_app_for_calendar().await;

    // User 2 queries November 2026 (should have 0 events)
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/calendar/events?year=2026&month=11")
                .header(COOKIE, format!("taskflow_session={}", token2))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let events = json["data"].as_array().unwrap();
    assert_eq!(events.len(), 0, "User 2 must not see User 1's calendar events");
}
