use axum::{
    body::{to_bytes, Body},
    http::{header::COOKIE, Request, StatusCode},
};
use serde_json::{json, Value};
use std::sync::Arc;
use taskflow::{
    config::Config,
    database::{create_connection_pool, run_migrations},
    routes::{create_router, AppState},
};
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_app_with_admin_and_user() -> (axum::Router, AppState, String, String, i64, i64) {
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

    // Admin User
    let admin_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (uuid, username, email, password_hash, role) VALUES (?, ?, ?, ?, ?) RETURNING id"
    )
    .bind(Uuid::new_v4().to_string())
    .bind("admin_boss")
    .bind("boss@taskflow.dev")
    .bind("hash")
    .bind("admin")
    .fetch_one(&pool)
    .await
    .unwrap();

    let admin_token = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sessions (token, user_id, expires_at) VALUES (?, ?, datetime('now', '+1 hour'))")
        .bind(&admin_token)
        .bind(admin_id)
        .execute(&pool)
        .await
        .unwrap();

    // Regular User
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (uuid, username, email, password_hash, role) VALUES (?, ?, ?, ?, ?) RETURNING id"
    )
    .bind(Uuid::new_v4().to_string())
    .bind("standard_user")
    .bind("std@taskflow.dev")
    .bind("hash")
    .bind("user")
    .fetch_one(&pool)
    .await
    .unwrap();

    let user_token = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sessions (token, user_id, expires_at) VALUES (?, ?, datetime('now', '+1 hour'))")
        .bind(&user_token)
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    (app, state, admin_token, user_token, admin_id, user_id)
}

#[tokio::test]
async fn test_admin_rbac_protection() {
    let (app, _, _, user_token, _, _) = setup_app_with_admin_and_user().await;

    // Regular user attempts to access /api/v1/admin/stats -> 403 Forbidden
    let stats_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/stats")
                .header(COOKIE, format!("taskflow_session={}", user_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(stats_res.status(), StatusCode::FORBIDDEN);

    // Regular user attempts to access /api/v1/admin/users -> 403 Forbidden
    let users_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/users")
                .header(COOKIE, format!("taskflow_session={}", user_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(users_res.status(), StatusCode::FORBIDDEN);

    // Unauthenticated request -> 401 Unauthorized
    let unauth_res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(unauth_res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_platform_stats_and_user_list() {
    let (app, _, admin_token, _, _, _) = setup_app_with_admin_and_user().await;

    // 1. Admin gets stats
    let stats_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/stats")
                .header(COOKIE, format!("taskflow_session={}", admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(stats_res.status(), StatusCode::OK);
    let body = to_bytes(stats_res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["total_users"], 2);
    assert_eq!(json["data"]["active_users"], 2);
    assert_eq!(json["data"]["active_sessions"], 2);

    // 2. Admin gets user list
    let users_res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/users")
                .header(COOKIE, format!("taskflow_session={}", admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(users_res.status(), StatusCode::OK);
    let u_body = to_bytes(users_res.into_body(), usize::MAX).await.unwrap();
    let u_json: Value = serde_json::from_slice(&u_body).unwrap();
    let users = u_json["data"].as_array().unwrap();
    assert_eq!(users.len(), 2);
}

#[tokio::test]
async fn test_admin_lock_user_and_invalidate_sessions() {
    let (app, state, admin_token, user_token, _, user_id) = setup_app_with_admin_and_user().await;

    // 1. Verify user can access /api/v1/auth/me initially
    let me_res1 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header(COOKIE, format!("taskflow_session={}", user_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(me_res1.status(), StatusCode::OK);

    // 2. Admin deactivates user
    let deactivate_payload = json!({ "is_active": false });
    let deact_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/admin/users/{}/status", user_id))
                .header(COOKIE, format!("taskflow_session={}", admin_token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&deactivate_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(deact_res.status(), StatusCode::OK);

    // 3. Verify user's session was invalidated in DB
    let session_count: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(session_count, 0, "User sessions must be wiped upon deactivation");

    // 4. Verify user can no longer access /api/v1/auth/me
    let me_res2 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header(COOKIE, format!("taskflow_session={}", user_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(me_res2.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_self_protection() {
    let (app, _, admin_token, _, admin_id, _) = setup_app_with_admin_and_user().await;

    // Admin attempts to deactivate themselves -> 400 Bad Request
    let self_deact_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/admin/users/{}/status", admin_id))
                .header(COOKIE, format!("taskflow_session={}", admin_token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&json!({ "is_active": false })).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(self_deact_res.status(), StatusCode::BAD_REQUEST);

    // Admin attempts to demote themselves -> 400 Bad Request
    let self_demote_res = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/admin/users/{}/role", admin_id))
                .header(COOKIE, format!("taskflow_session={}", admin_token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&json!({ "role": "user" })).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(self_demote_res.status(), StatusCode::BAD_REQUEST);
}
