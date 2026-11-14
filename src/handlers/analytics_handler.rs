use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use crate::{
    errors::AppError,
    middleware::AuthUser,
    routes::AppState,
    services::AnalyticsService,
};

pub async fn get_dashboard_overview(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let overview = AnalyticsService::get_dashboard_overview(&state.db, user.id).await?;

    Ok(Json(json!({
        "success": true,
        "data": overview
    })))
}

pub async fn get_productivity_report(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let report = AnalyticsService::get_productivity_report(&state.db, user.id).await?;

    Ok(Json(json!({
        "success": true,
        "data": report
    })))
}
