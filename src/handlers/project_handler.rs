use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::{
    errors::AppError,
    middleware::AuthUser,
    routes::AppState,
    services::{CreateProjectDto, ProjectService, UpdateProjectDto},
};

pub async fn create_project(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(payload): Json<CreateProjectDto>,
) -> Result<impl IntoResponse, AppError> {
    let project = ProjectService::create_project(&state.db, user.id, payload).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "Project created successfully.",
            "data": project
        })),
    ))
}

pub async fn list_projects(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let projects = ProjectService::list_projects(&state.db, user.id).await?;

    Ok(Json(json!({
        "success": true,
        "data": projects
    })))
}

pub async fn get_project(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let project = ProjectService::get_project_by_id(&state.db, user.id, id).await?;

    Ok(Json(json!({
        "success": true,
        "data": project
    })))
}

pub async fn get_project_detail(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let detail = ProjectService::get_project_detail(&state.db, user.id, id).await?;

    Ok(Json(json!({
        "success": true,
        "data": detail
    })))
}

pub async fn update_project(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateProjectDto>,
) -> Result<impl IntoResponse, AppError> {
    let project = ProjectService::update_project(&state.db, user.id, id, payload).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Project updated successfully.",
        "data": project
    })))
}

pub async fn delete_project(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    ProjectService::delete_project(&state.db, user.id, id).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Project deleted successfully."
    })))
}
