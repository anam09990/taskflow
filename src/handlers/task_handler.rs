use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::{
    errors::AppError,
    middleware::AuthUser,
    routes::AppState,
    services::{CreateTaskDto, TaskFilterQuery, TaskService, UpdateTaskDto, UpdateTaskStatusDto},
};

pub async fn create_task(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(payload): Json<CreateTaskDto>,
) -> Result<impl IntoResponse, AppError> {
    let task = TaskService::create_task(&state.db, user.id, payload).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "Task created successfully.",
            "data": task
        })),
    ))
}

pub async fn list_tasks(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(filter): Query<TaskFilterQuery>,
) -> Result<impl IntoResponse, AppError> {
    let tasks = TaskService::list_tasks(&state.db, user.id, filter).await?;

    Ok(Json(json!({
        "success": true,
        "data": tasks
    })))
}

pub async fn get_task(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let task = TaskService::get_task_by_id(&state.db, user.id, id).await?;

    Ok(Json(json!({
        "success": true,
        "data": task
    })))
}

pub async fn update_task(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateTaskDto>,
) -> Result<impl IntoResponse, AppError> {
    let task = TaskService::update_task(&state.db, user.id, id, payload).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Task updated successfully.",
        "data": task
    })))
}

pub async fn update_task_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateTaskStatusDto>,
) -> Result<impl IntoResponse, AppError> {
    let task = TaskService::update_task_status(&state.db, user.id, id, payload.status).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Task status updated.",
        "data": task
    })))
}

pub async fn delete_task(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    TaskService::delete_task(&state.db, user.id, id).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Task deleted successfully."
    })))
}
