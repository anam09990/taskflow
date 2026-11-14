use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::{
    errors::AppError,
    middleware::AuthUser,
    routes::AppState,
    services::{CalendarQuery, CalendarService},
};

pub async fn get_calendar_events(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(query): Query<CalendarQuery>,
) -> Result<impl IntoResponse, AppError> {
    let events = CalendarService::get_calendar_events(&state.db, user.id, query).await?;

    Ok(Json(json!({
        "success": true,
        "data": events
    })))
}
