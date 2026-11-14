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

async fn setup_app_with_two_users() -> (axum::Router, AppState, String, String) {
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
    .bind("p_user1")
    .bind("p_user1@example.com")
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
    .bind("p_user2")
    .bind("p_user2@example.com")
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
async fn test_project_crud_and_progress_tracking() {
    let (app, _, token1, _) = setup_app_with_two_users().await;

    // 1. Create Project
    let create_payload = json!({
        "title": "Mobile App V2",
        "description": "Redesign mobile app with Rust backend",
        "color": "#10b981",
        "status": "active"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/projects")
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
    let project_id = json["data"]["id"].as_i64().unwrap();
    assert_eq!(json["data"]["title"], "Mobile App V2");
    assert_eq!(json["data"]["color"], "#10b981");

    // 2. Create 2 tasks assigned to this project
    let task1_payload = json!({
        "title": "Build API Client",
        "project_id": project_id,
        "priority": "high",
        "status": "done"
    });

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header(COOKIE, format!("taskflow_session={}", token1))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&task1_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let task2_payload = json!({
        "title": "Design Screens",
        "project_id": project_id,
        "priority": "medium",
        "status": "in_progress"
    });

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header(COOKIE, format!("taskflow_session={}", token1))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&task2_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // 3. List projects and check aggregated metrics
    let list_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/projects")
                .header(COOKIE, format!("taskflow_session={}", token1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_res.status(), StatusCode::OK);
    let list_body = to_bytes(list_res.into_body(), usize::MAX).await.unwrap();
    let list_json: Value = serde_json::from_slice(&list_body).unwrap();
    let projects = list_json["data"].as_array().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["total_tasks"], 2);
    assert_eq!(projects[0]["completed_tasks"], 1);

    // 4. Get Project Detail with associated tasks
    let detail_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/projects/{}/detail", project_id))
                .header(COOKIE, format!("taskflow_session={}", token1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(detail_res.status(), StatusCode::OK);
    let detail_body = to_bytes(detail_res.into_body(), usize::MAX).await.unwrap();
    let detail_json: Value = serde_json::from_slice(&detail_body).unwrap();
    let tasks = detail_json["data"]["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 2);

    // 5. Update Project
    let update_payload = json!({
        "title": "Mobile App V2.1",
        "color": "#8b5cf6"
    });

    let update_res = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/projects/{}", project_id))
                .header(COOKIE, format!("taskflow_session={}", token1))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_project_deletion_preserves_tasks() {
    let (app, state, token1, _) = setup_app_with_two_users().await;

    // Create project
    let project_payload = json!({ "title": "Temporary Project" });
    let p_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/projects")
                .header(COOKIE, format!("taskflow_session={}", token1))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&project_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let p_body = to_bytes(p_res.into_body(), usize::MAX).await.unwrap();
    let p_json: Value = serde_json::from_slice(&p_body).unwrap();
    let project_id = p_json["data"]["id"].as_i64().unwrap();

    // Create task assigned to project
    let task_payload = json!({
        "title": "Assigned Task",
        "project_id": project_id
    });

    let t_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header(COOKIE, format!("taskflow_session={}", token1))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&task_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let t_body = to_bytes(t_res.into_body(), usize::MAX).await.unwrap();
    let t_json: Value = serde_json::from_slice(&t_body).unwrap();
    let task_id = t_json["data"]["id"].as_i64().unwrap();

    // Delete project
    let del_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/projects/{}", project_id))
                .header(COOKIE, format!("taskflow_session={}", token1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(del_res.status(), StatusCode::OK);

    // Verify task still exists and its project_id is now NULL
    let task_row: (Option<i64>,) = sqlx::query_as("SELECT project_id FROM tasks WHERE id = ?")
        .bind(task_id)
        .fetch_one(&state.db)
        .await
        .unwrap();

    assert_eq!(task_row.0, None, "Task project_id must become NULL upon project deletion");
}

#[tokio::test]
async fn test_project_cross_user_isolation() {
    let (app, _, token1, token2) = setup_app_with_two_users().await;

    // User 1 creates project
    let p_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/projects")
                .header(COOKIE, format!("taskflow_session={}", token1))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&json!({ "title": "User 1 Secret Project" })).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let p_body = to_bytes(p_res.into_body(), usize::MAX).await.unwrap();
    let p_json: Value = serde_json::from_slice(&p_body).unwrap();
    let project_id = p_json["data"]["id"].as_i64().unwrap();

    // User 2 tries to GET User 1's project -> 404
    let get_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/projects/{}", project_id))
                .header(COOKIE, format!("taskflow_session={}", token2))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_res.status(), StatusCode::NOT_FOUND);

    // User 2 tries to DELETE User 1's project -> 404
    let del_res = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/projects/{}", project_id))
                .header(COOKIE, format!("taskflow_session={}", token2))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(del_res.status(), StatusCode::NOT_FOUND);
}
