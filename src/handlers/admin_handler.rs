use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::{
    errors::AppError,
    middleware::AdminUser,
    routes::AppState,
    services::{AdminService, UpdateUserRoleDto, UpdateUserStatusDto},
};

pub async fn get_platform_stats(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> Result<impl IntoResponse, AppError> {
    let stats = AdminService::get_platform_stats(&state.db).await?;

    Ok(Json(json!({
        "success": true,
        "data": stats
    })))
}

pub async fn list_users(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> Result<impl IntoResponse, AppError> {
    let users = AdminService::list_users(&state.db).await?;

    Ok(Json(json!({
        "success": true,
        "data": users
    })))
}

pub async fn update_user_status(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(target_id): Path<i64>,
    Json(payload): Json<UpdateUserStatusDto>,
) -> Result<impl IntoResponse, AppError> {
    AdminService::update_user_status(&state.db, admin.id, target_id, payload.is_active).await?;

    Ok(Json(json!({
        "success": true,
        "message": "User status updated successfully."
    })))
}

pub async fn update_user_role(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(target_id): Path<i64>,
    Json(payload): Json<UpdateUserRoleDto>,
) -> Result<impl IntoResponse, AppError> {
    AdminService::update_user_role(&state.db, admin.id, target_id, payload.role).await?;

    Ok(Json(json!({
        "success": true,
        "message": "User role updated successfully."
    })))
}
