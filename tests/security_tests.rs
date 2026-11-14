use axum::{
    body::{to_bytes, Body},
    http::{header::{AUTHORIZATION, COOKIE}, Request, StatusCode},
};
use chrono::{Duration, Utc};
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
async fn test_sql_injection_defense() {
    let (app, state) = setup_test_app().await;

    // Register a valid user
    let reg_payload = json!({
        "username": "legituser",
        "email": "legit@taskflow.dev",
        "password": "LegitPassword123!"
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

    // 1. SQL Injection in login username
    let sqli_login = json!({
        "username_or_email": "legituser' OR '1'='1",
        "password": "any_password"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&sqli_login).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 2. SQL Injection with comment syntax
    let sqli_comment = json!({
        "username_or_email": "legituser' --",
        "password": "any_password"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&sqli_comment).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Log in legit user to get session
    let login_payload = json!({
        "username_or_email": "legituser",
        "password": "LegitPassword123!"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let token = json["data"]["token"].as_str().unwrap().to_string();

    // 3. SQL Injection payload in task creation
    let sqli_task = json!({
        "title": "'); DROP TABLE tasks; --",
        "description": "SQL injection test in description",
        "priority": "high",
        "status": "todo"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header("content-type", "application/json")
                .header(AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::from(serde_json::to_vec(&sqli_task).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // Verify tasks table was NOT dropped and task was inserted literally
    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM tasks")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(count.0, 1);

    // 4. SQL Injection payload in search query
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks?search=%27+OR+1%3D1--")
                .header(AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_xss_and_input_boundaries() {
    let (app, _) = setup_test_app().await;

    // 1. Input validation limits: username too short
    let short_user = json!({
        "username": "ab",
        "email": "test@taskflow.dev",
        "password": "Password123!"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&short_user).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 2. Input validation limits: invalid email
    let invalid_email = json!({
        "username": "validuser",
        "email": "not-an-email",
        "password": "Password123!"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&invalid_email).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Register valid user
    let valid_user = json!({
        "username": "xssuser",
        "email": "xss@taskflow.dev",
        "password": "Password123!"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&valid_user).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // Login
    let login = json!({
        "username_or_email": "xssuser",
        "password": "Password123!"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&login).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let token = json["data"]["token"].as_str().unwrap().to_string();

    // 3. Stored XSS payload in task title & description
    let xss_payload = json!({
        "title": "<script>alert('XSS-Title')</script>",
        "description": "<img src=x onerror=\"alert('XSS-Desc')\" />",
        "priority": "urgent",
        "status": "todo"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header("content-type", "application/json")
                .header(AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::from(serde_json::to_vec(&xss_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["data"]["title"], "<script>alert('XSS-Title')</script>");
}

#[tokio::test]
async fn test_account_enumeration_defense() {
    let (app, _) = setup_test_app().await;

    // Register user
    let reg = json!({
        "username": "targetuser",
        "email": "target@taskflow.dev",
        "password": "TargetPassword123!"
    });
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&reg).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // 1. Attempt login with non-existent user
    let nonexistent = json!({
        "username_or_email": "nobody_exists",
        "password": "SomePassword123!"
    });
    let res_nonexistent = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&nonexistent).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_nonexistent.status(), StatusCode::UNAUTHORIZED);
    let body1 = to_bytes(res_nonexistent.into_body(), usize::MAX).await.unwrap();
    let json1: Value = serde_json::from_slice(&body1).unwrap();

    // 2. Attempt login with real user but wrong password
    let wrong_pw = json!({
        "username_or_email": "targetuser",
        "password": "WrongPassword999!"
    });
    let res_wrong_pw = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&wrong_pw).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_wrong_pw.status(), StatusCode::UNAUTHORIZED);
    let body2 = to_bytes(res_wrong_pw.into_body(), usize::MAX).await.unwrap();
    let json2: Value = serde_json::from_slice(&body2).unwrap();

    // Confirm EXACT matching error message to prevent user enumeration
    assert_eq!(json1["error"], json2["error"]);
    assert_eq!(json1["error"]["message"], "Invalid username/email or password.");
}

#[tokio::test]
async fn test_privilege_escalation_defense() {
    let (app, _) = setup_test_app().await;

    // User 1 is admin
    let admin_payload = json!({
        "username": "admin1",
        "email": "admin1@taskflow.dev",
        "password": "Password123!"
    });
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&admin_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // User 2 is regular user
    let user_payload = json!({
        "username": "regularuser",
        "email": "user@taskflow.dev",
        "password": "Password123!"
    });
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&user_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Login as regular user
    let login = json!({
        "username_or_email": "regularuser",
        "password": "Password123!"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&login).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let user_token = json["data"]["token"].as_str().unwrap().to_string();

    // Attempt to access admin stats
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/stats")
                .header(AUTHORIZATION, format!("Bearer {}", user_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // Attempt to access admin users list
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/users")
                .header(AUTHORIZATION, format!("Bearer {}", user_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // Attempt to promote self to admin
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/admin/users/2/role")
                .header("content-type", "application/json")
                .header(AUTHORIZATION, format!("Bearer {}", user_token))
                .body(Body::from(serde_json::to_vec(&json!({ "role": "admin" })).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_session_tampering_and_expiration() {
    let (app, state) = setup_test_app().await;

    // Register user
    let reg = json!({
        "username": "sessionuser",
        "email": "session@taskflow.dev",
        "password": "Password123!"
    });
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&reg).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // 1. Forged session token
    let fake_token = "00000000-0000-0000-0000-000000000000";
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header(AUTHORIZATION, format!("Bearer {}", fake_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 2. Cookie tampering
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header(COOKIE, "taskflow_session=invalid_random_cookie_value")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 3. Expired session token
    let expired_token = uuid::Uuid::new_v4().to_string();
    let expired_time = Utc::now() - Duration::hours(2);
    sqlx::query(
        "INSERT INTO sessions (token, user_id, expires_at) VALUES (?, 1, ?)"
    )
    .bind(&expired_token)
    .bind(expired_time)
    .execute(&state.db)
    .await
    .unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header(AUTHORIZATION, format!("Bearer {}", expired_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_security_headers_enforcement() {
    let (app, _) = setup_test_app().await;

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let headers = res.headers();

    assert_eq!(
        headers.get("x-content-type-options").map(|v| v.to_str().unwrap()),
        Some("nosniff")
    );
    assert_eq!(
        headers.get("x-frame-options").map(|v| v.to_str().unwrap()),
        Some("DENY")
    );
    assert_eq!(
        headers.get("referrer-policy").map(|v| v.to_str().unwrap()),
        Some("strict-origin-when-cross-origin")
    );
    assert!(headers.get("content-security-policy").is_some());
    assert!(headers.get("permissions-policy").is_some());
}
