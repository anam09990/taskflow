use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::errors::AppError;

#[derive(Debug, Serialize)]
pub struct PlatformStats {
    pub total_users: i64,
    pub active_users: i64,
    pub active_sessions: i64,
    pub total_tasks: i64,
    pub total_projects: i64,
    pub system_version: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminUserItem {
    pub id: i64,
    pub uuid: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub is_active: bool,
    pub task_count: i64,
    pub project_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserStatusDto {
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRoleDto {
    pub role: String,
}

pub struct AdminService;

impl AdminService {
    pub async fn get_platform_stats(pool: &SqlitePool) -> Result<PlatformStats, AppError> {
        let total_users: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
            .fetch_one(pool)
            .await?;

        let active_users: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE is_active = 1")
            .fetch_one(pool)
            .await?;

        let active_sessions: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sessions WHERE datetime(expires_at) > datetime('now')"
        )
        .fetch_one(pool)
        .await?;

        let total_tasks: i64 = sqlx::query_scalar("SELECT count(*) FROM tasks")
            .fetch_one(pool)
            .await?;

        let total_projects: i64 = sqlx::query_scalar("SELECT count(*) FROM projects")
            .fetch_one(pool)
            .await?;

        Ok(PlatformStats {
            total_users,
            active_users,
            active_sessions,
            total_tasks,
            total_projects,
            system_version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    pub async fn list_users(pool: &SqlitePool) -> Result<Vec<AdminUserItem>, AppError> {
        let users = sqlx::query_as::<_, AdminUserItem>(
            r#"
            SELECT u.id, u.uuid, u.username, u.email, u.role, u.is_active, u.created_at,
                   COALESCE((SELECT count(*) FROM tasks t WHERE t.user_id = u.id), 0) as task_count,
                   COALESCE((SELECT count(*) FROM projects p WHERE p.user_id = u.id), 0) as project_count
            FROM users u
            ORDER BY u.created_at ASC
            "#
        )
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    pub async fn update_user_status(
        pool: &SqlitePool,
        admin_user_id: i64,
        target_user_id: i64,
        is_active: bool,
    ) -> Result<(), AppError> {
        if admin_user_id == target_user_id && !is_active {
            return Err(AppError::BadRequest("You cannot deactivate your own administrative account.".to_string()));
        }

        let existing: Option<(String,)> = sqlx::query_as("SELECT username FROM users WHERE id = ?")
            .bind(target_user_id)
            .fetch_optional(pool)
            .await?;

        let username = existing.ok_or_else(|| AppError::NotFound("User not found.".to_string()))?.0;

        sqlx::query("UPDATE users SET is_active = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(is_active)
            .bind(target_user_id)
            .execute(pool)
            .await?;

        // If deactivated, revoke all active sessions immediately
        if !is_active {
            sqlx::query("DELETE FROM sessions WHERE user_id = ?")
                .bind(target_user_id)
                .execute(pool)
                .await?;
        }

        // Log admin audit action
        let _ = sqlx::query(
            "INSERT INTO activity_logs (user_id, action, entity_type, entity_id, details) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(admin_user_id)
        .bind("admin_update_status")
        .bind("user")
        .bind(target_user_id.to_string())
        .bind(format!("Admin changed user \"{}\" status to {}", username, if is_active { "Active" } else { "Deactivated" }))
        .execute(pool)
        .await;

        Ok(())
    }

    pub async fn update_user_role(
        pool: &SqlitePool,
        admin_user_id: i64,
        target_user_id: i64,
        role: String,
    ) -> Result<(), AppError> {
        let role = role.to_lowercase();
        if !["admin", "user"].contains(&role.as_str()) {
            return Err(AppError::BadRequest("Role must be 'admin' or 'user'.".to_string()));
        }

        if admin_user_id == target_user_id && role != "admin" {
            return Err(AppError::BadRequest("You cannot demote your own administrative role.".to_string()));
        }

        let existing: Option<(String,)> = sqlx::query_as("SELECT username FROM users WHERE id = ?")
            .bind(target_user_id)
            .fetch_optional(pool)
            .await?;

        let username = existing.ok_or_else(|| AppError::NotFound("User not found.".to_string()))?.0;

        sqlx::query("UPDATE users SET role = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(&role)
            .bind(target_user_id)
            .execute(pool)
            .await?;

        // Log admin audit action
        let _ = sqlx::query(
            "INSERT INTO activity_logs (user_id, action, entity_type, entity_id, details) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(admin_user_id)
        .bind("admin_update_role")
        .bind("user")
        .bind(target_user_id.to_string())
        .bind(format!("Admin changed user \"{}\" role to {}", username, role))
        .execute(pool)
        .await;

        Ok(())
    }
}
