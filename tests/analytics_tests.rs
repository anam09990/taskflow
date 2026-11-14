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

async fn setup_app_for_analytics() -> (axum::Router, AppState, String, String) {
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

    // User 1
    let user1_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (uuid, username, email, password_hash, role) VALUES (?, ?, ?, ?, ?) RETURNING id"
    )
    .bind(Uuid::new_v4().to_string())
    .bind("an_user1")
    .bind("an1@example.com")
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
    .bind("an_user2")
    .bind("an2@example.com")
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

    // User 1 Seed Data:
    // 1. Completed task today
    sqlx::query("INSERT INTO tasks (uuid, user_id, title, priority, status, completed_at) VALUES (?, ?, ?, ?, ?, datetime('now'))")
        .bind(Uuid::new_v4().to_string())
        .bind(user1_id)
        .bind("Task Completed Today")
        .bind("urgent")
        .bind("done")
        .execute(&pool)
        .await
        .unwrap();

    // 2. Overdue pending task (due 3 days ago)
    sqlx::query("INSERT INTO tasks (uuid, user_id, title, priority, status, due_date) VALUES (?, ?, ?, ?, ?, datetime('now', '-3 days'))")
        .bind(Uuid::new_v4().to_string())
        .bind(user1_id)
        .bind("Overdue Task")
        .bind("high")
        .bind("todo")
        .execute(&pool)
        .await
        .unwrap();

    // 3. Regular active task
    sqlx::query("INSERT INTO tasks (uuid, user_id, title, priority, status) VALUES (?, ?, ?, ?, ?)")
        .bind(Uuid::new_v4().to_string())
        .bind(user1_id)
        .bind("Active Task")
        .bind("medium")
        .bind("in_progress")
        .execute(&pool)
        .await
        .unwrap();

    (app, state, token1, token2)
}

#[tokio::test]
async fn test_productivity_report_calculations() {
    let (app, _, token1, _) = setup_app_for_analytics().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/analytics/report")
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
    assert_eq!(json["data"]["total_tasks"], 3);
    assert_eq!(json["data"]["completed_tasks"], 1);
    assert_eq!(json["data"]["pending_tasks"], 2);
    assert_eq!(json["data"]["overdue_tasks"], 1, "Exactly one task is overdue");
    assert_eq!(json["data"]["weekly_completed_count"], 1);

    let weekly_vel = json["data"]["weekly_velocity"].as_array().unwrap();
    assert_eq!(weekly_vel.len(), 7, "Weekly velocity must contain 7 days");

    let monthly_vel = json["data"]["monthly_velocity"].as_array().unwrap();
    assert_eq!(monthly_vel.len(), 30, "Monthly velocity must contain 30 days");

    let prio = json["data"]["priority_distribution"].as_array().unwrap();
    assert_eq!(prio.len(), 4, "Must contain all 4 priority tiers");
}

#[tokio::test]
async fn test_analytics_cross_user_isolation() {
    let (app, _, _, token2) = setup_app_for_analytics().await;

    // User 2 has 0 tasks
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/analytics/report")
                .header(COOKIE, format!("taskflow_session={}", token2))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["total_tasks"], 0);
    assert_eq!(json["data"]["completed_tasks"], 0);
    assert_eq!(json["data"]["overdue_tasks"], 0);
}
