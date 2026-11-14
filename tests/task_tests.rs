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

async fn setup_app_with_users() -> (axum::Router, AppState, String, String) {
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
    .bind("user1")
    .bind("user1@example.com")
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
    .bind("user2")
    .bind("user2@example.com")
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

    (app, state, token1, token2)
}

#[tokio::test]
async fn test_task_crud_lifecycle() {
    let (app, _, token1, _) = setup_app_with_users().await;

    // 1. Create Task
    let create_payload = json!({
        "title": "Complete Rust Web Architecture",
        "description": "Implement Phase 6 tasks module",
        "priority": "urgent",
        "status": "todo"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header(COOKIE, format!("taskflow_session={}", token1))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&create_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    let task_id = json["data"]["id"].as_i64().unwrap();
    assert_eq!(json["data"]["priority"], "urgent");
    assert_eq!(json["data"]["status"], "todo");

    // 2. Get Task by ID
    let get_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/tasks/{}", task_id))
                .header(COOKIE, format!("taskflow_session={}", token1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_res.status(), StatusCode::OK);

    // 3. Update Task Status to 'done' (PATCH)
    let patch_payload = json!({ "status": "done" });
    let patch_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/tasks/{}/status", task_id))
                .header(COOKIE, format!("taskflow_session={}", token1))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&patch_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(patch_res.status(), StatusCode::OK);
    let patch_body = to_bytes(patch_res.into_body(), usize::MAX).await.unwrap();
    let patch_json: Value = serde_json::from_slice(&patch_body).unwrap();
    assert_eq!(patch_json["data"]["status"], "done");
    assert!(patch_json["data"]["completed_at"].is_string());

    // 4. Delete Task
    let del_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/tasks/{}", task_id))
                .header(COOKIE, format!("taskflow_session={}", token1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(del_res.status(), StatusCode::OK);

    // 5. Verify Task is gone
    let verify_res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/tasks/{}", task_id))
                .header(COOKIE, format!("taskflow_session={}", token1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(verify_res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_task_filters_and_search() {
    let (app, _, token1, _) = setup_app_with_users().await;

    // Create 3 tasks
    let tasks_data = [
        ("Design Wireframes", "high", "done"),
        ("Write Unit Tests", "medium", "in_progress"),
        ("Review PR", "low", "todo"),
    ];

    for (title, priority, status) in tasks_data {
        let payload = json!({
            "title": title,
            "priority": priority,
            "status": status
        });

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tasks")
                    .header(COOKIE, format!("taskflow_session={}", token1))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // 1. Filter by status=done
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks?status=done")
                .header(COOKIE, format!("taskflow_session={}", token1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let items = json["data"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["title"], "Design Wireframes");

    // 2. Search by keyword "Unit"
    let search_res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks?search=Unit")
                .header(COOKIE, format!("taskflow_session={}", token1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(search_res.status(), StatusCode::OK);
    let search_body = to_bytes(search_res.into_body(), usize::MAX).await.unwrap();
    let search_json: Value = serde_json::from_slice(&search_body).unwrap();
    let search_items = search_json["data"].as_array().unwrap();
    assert_eq!(search_items.len(), 1);
    assert_eq!(search_items[0]["title"], "Write Unit Tests");
}

#[tokio::test]
async fn test_task_authorization_isolation() {
    let (app, _, token1, token2) = setup_app_with_users().await;

    // User 1 creates task
    let payload = json!({
        "title": "Confidential User1 Task",
        "priority": "high",
        "status": "todo"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header(COOKIE, format!("taskflow_session={}", token1))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let task_id = json["data"]["id"].as_i64().unwrap();

    // User 2 tries to GET User 1's task (must return 404 Not Found)
    let get_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/tasks/{}", task_id))
                .header(COOKIE, format!("taskflow_session={}", token2))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_res.status(), StatusCode::NOT_FOUND);

    // User 2 tries to DELETE User 1's task (must return 404 Not Found)
    let del_res = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/tasks/{}", task_id))
                .header(COOKIE, format!("taskflow_session={}", token2))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(del_res.status(), StatusCode::NOT_FOUND);
}
