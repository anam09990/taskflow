use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use crate::{
    errors::AppError,
    models::UserResponse,
    routes::AppState,
    services::AuthService,
};

pub struct AuthUser(pub UserResponse);

pub struct AdminUser(pub UserResponse);

pub struct OptionalAuthUser(pub Option<UserResponse>);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let token = extract_token(parts)
            .ok_or_else(|| AppError::Unauthorized("Authentication required. Please sign in.".to_string()))?;

        let user = AuthService::get_user_by_session(&state.db, &token).await?;
        Ok(AuthUser(user))
    }
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let AuthUser(user) = AuthUser::from_request_parts(parts, state).await?;

        if user.role != "admin" {
            return Err(AppError::Forbidden(
                "Access denied. Administrator privileges required.".to_string(),
            ));
        }

        Ok(AdminUser(user))
    }
}

impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        if let Some(token) = extract_token(parts) {
            if let Ok(user) = AuthService::get_user_by_session(&state.db, &token).await {
                return Ok(OptionalAuthUser(Some(user)));
            }
        }
        Ok(OptionalAuthUser(None))
    }
}

pub fn extract_token(parts: &Parts) -> Option<String> {
    // 1. Check Authorization Bearer header
    if let Some(auth_header) = parts.headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.trim().to_string());
            }
        }
    }

    // 2. Check Cookie header (taskflow_session=...)
    if let Some(cookie_header) = parts.headers.get("cookie") {
        if let Ok(cookies_str) = cookie_header.to_str() {
            for cookie in cookies_str.split(';') {
                let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                if parts.len() == 2 && parts[0] == "taskflow_session" {
                    return Some(parts[1].to_string());
                }
            }
        }
    }

    None
}
