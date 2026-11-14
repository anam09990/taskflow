use axum::{
    body::{to_bytes, Body},
    http::{header::{AUTHORIZATION, COOKIE, SET_COOKIE}, Request, StatusCode},
};
use serde_json::{json, Value};
use std::sync::Arc;
use taskflow::{
    config::Config,
    database::{create_connection_pool, run_migrations},
    routes::{create_router, AppState},
};
use tower::ServiceExt;

async fn setup_test_app() -> (axum::Router, AppState) {
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
        db: pool,
        config,
    };

    let app = create_router(state.clone());
    (app, state)
}

#[tokio::test]
async fn test_user_registration_flow() {
    let (app, _) = setup_test_app().await;

    // 1. First user registration (should always get regular "user" role)
    let reg_payload = json!({
        "username": "firstuser",
        "email": "first@taskflow.dev",
        "password": "Password123!"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&reg_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["username"], "firstuser");
    assert_eq!(json["data"]["role"], "user");

    // 2. Second user registration (should be regular user)
    let second_payload = json!({
        "username": "regularuser",
        "email": "user@taskflow.dev",
        "password": "Password123!"
    });

    let res2 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&second_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res2.status(), StatusCode::CREATED);
    let body2 = to_bytes(res2.into_body(), usize::MAX).await.unwrap();
    let json2: Value = serde_json::from_slice(&body2).unwrap();
    assert_eq!(json2["data"]["role"], "user");

    // 3. Duplicate registration attempt (should return 409 Conflict)
    let dup_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&reg_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(dup_res.status(), StatusCode::CONFLICT);

    // 4. Invalid input (password too short, should return 422 Unprocessable Entity)
    let short_pass_payload = json!({
        "username": "shortpass",
        "email": "short@example.com",
        "password": "123"
    });

    let invalid_res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&short_pass_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(invalid_res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_login_logout_and_session_extraction() {
    let (app, _) = setup_test_app().await;

    // Register user
    let reg_payload = json!({
        "username": "testuser",
        "email": "test@taskflow.dev",
        "password": "SecurePassword123!"
    });

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&reg_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // 1. Invalid login attempt
    let invalid_login = json!({
        "username_or_email": "testuser",
        "password": "WrongPassword!"
    });

    let bad_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&invalid_login).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(bad_res.status(), StatusCode::UNAUTHORIZED);

    // 2. Successful login
    let valid_login = json!({
        "username_or_email": "testuser",
        "password": "SecurePassword123!"
    });

    let login_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&valid_login).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(login_res.status(), StatusCode::OK);

    // Verify Set-Cookie header
    let cookie_header = login_res.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
    assert!(cookie_header.contains("taskflow_session="));
    assert!(cookie_header.contains("HttpOnly"));

    let body = to_bytes(login_res.into_body(), usize::MAX).await.unwrap();
    let login_json: Value = serde_json::from_slice(&body).unwrap();
    let token = login_json["data"]["token"].as_str().unwrap();

    // 3. Access protected /me route with cookie
    let me_cookie_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header(COOKIE, format!("taskflow_session={}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(me_cookie_res.status(), StatusCode::OK);
    let me_body = to_bytes(me_cookie_res.into_body(), usize::MAX).await.unwrap();
    let me_json: Value = serde_json::from_slice(&me_body).unwrap();
    assert_eq!(me_json["data"]["username"], "testuser");

    // 4. Access protected /me route with Authorization: Bearer token
    let me_bearer_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header(AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(me_bearer_res.status(), StatusCode::OK);

    // 5. Access protected /me route with invalid token
    let bad_token_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header(AUTHORIZATION, "Bearer invalid-uuid-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(bad_token_res.status(), StatusCode::UNAUTHORIZED);

    // 6. Logout
    let logout_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header(COOKIE, format!("taskflow_session={}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(logout_res.status(), StatusCode::OK);
    let logout_cookie = logout_res.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
    assert!(logout_cookie.contains("Max-Age=0"));

    // 7. Verify session is no longer valid after logout
    let after_logout_res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header(COOKIE, format!("taskflow_session={}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(after_logout_res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_seed_on_startup() {
    let pool = create_connection_pool("sqlite::memory:")
        .await
        .expect("Failed to create in-memory db pool");

    run_migrations(&pool).await.expect("Migrations failed");

    let config = Config {
        server_host: "127.0.0.1".to_string(),
        server_port: 3000,
        database_url: "sqlite::memory:".to_string(),
        session_secret: "test-secret-at-least-32-chars-long-security-check".to_string(),
        session_max_age_secs: 3600,
        environment: "test".to_string(),
        admin_username: Some("systemadmin".to_string()),
        admin_email: Some("admin@taskflow.dev".to_string()),
        admin_password: Some("SuperSecretPassword123!".to_string()),
    };

    // 1. Seed admin
    taskflow::services::AuthService::seed_admin(&pool, &config)
        .await
        .expect("Admin seed should succeed");

    // Verify admin user exists in database with admin role
    let user: taskflow::models::User = sqlx::query_as("SELECT * FROM users WHERE username = ?")
        .bind("systemadmin")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(user.username, "systemadmin");
    assert_eq!(user.email, "admin@taskflow.dev");
    assert_eq!(user.role, "admin");

    // 2. Calling seed_admin again should be idempotent
    taskflow::services::AuthService::seed_admin(&pool, &config)
        .await
        .expect("Second admin seed call should be idempotent");

    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM users WHERE username = ?")
        .bind("systemadmin")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);

    // 3. Verify admin can authenticate with seeded password
    let login_dto = taskflow::services::auth_service::LoginDto {
        username_or_email: "admin@taskflow.dev".to_string(),
        password: "SuperSecretPassword123!".to_string(),
    };
    let auth = taskflow::services::AuthService::login(&pool, login_dto, 3600, None)
        .await
        .expect("Admin login should succeed");
    assert_eq!(auth.user.role, "admin");
}
