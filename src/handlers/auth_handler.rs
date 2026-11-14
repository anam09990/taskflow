use axum::{
    extract::State,
    http::{header::SET_COOKIE, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{
    errors::AppError,
    middleware::{extract_token, AuthUser},
    routes::AppState,
    services::{AuthService, LoginDto, RegisterDto},
};

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> Result<impl IntoResponse, AppError> {
    let user = AuthService::register(&state.db, payload).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "User registered successfully.",
            "data": user
        })),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginDto>,
) -> Result<Response, AppError> {
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let auth_res = AuthService::login(
        &state.db,
        payload,
        state.config.session_max_age_secs,
        user_agent,
    )
    .await?;

    let secure_flag = if state.config.environment == "production" { "; Secure" } else { "" };
    let cookie = format!(
        "taskflow_session={}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}{}",
        auth_res.token, state.config.session_max_age_secs, secure_flag
    );

    let mut response = (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Login successful.",
            "data": {
                "user": auth_res.user,
                "token": auth_res.token,
                "expires_at": auth_res.expires_at
            }
        })),
    )
        .into_response();

    response.headers_mut().insert(
        SET_COOKIE,
        cookie.parse().map_err(|e| {
            AppError::InternalServerError(format!("Failed to format cookie header: {}", e))
        })?,
    );

    Ok(response)
}

pub async fn logout(
    State(state): State<AppState>,
    parts: axum::http::request::Parts,
) -> Result<Response, AppError> {
    if let Some(token) = extract_token(&parts) {
        let _ = AuthService::logout(&state.db, &token).await;
    }

    let secure_flag = if state.config.environment == "production" { "; Secure" } else { "" };
    let cookie = format!("taskflow_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0{}", secure_flag);

    let mut response = (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Logged out successfully."
        })),
    )
        .into_response();

    response.headers_mut().insert(
        SET_COOKIE,
        cookie.parse().map_err(|e| {
            AppError::InternalServerError(format!("Failed to format cookie header: {}", e))
        })?,
    );

    Ok(response)
}

pub async fn me(AuthUser(user): AuthUser) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": user
        })),
    )
}
