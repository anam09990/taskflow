use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;
use validator::Validate;

use crate::{
    errors::AppError,
    models::{Project, ProjectWithStats, TaskWithProject},
};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProjectDto {
    #[validate(length(min = 1, max = 100, message = "Project title must be between 1 and 100 characters"))]
    pub title: String,

    pub description: Option<String>,
    pub color: Option<String>,
    pub status: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProjectDto {
    #[validate(length(min = 1, max = 100, message = "Project title must be between 1 and 100 characters"))]
    pub title: Option<String>,

    pub description: Option<String>,
    pub color: Option<String>,
    pub status: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ProjectDetailResponse {
    pub project: ProjectWithStats,
    pub tasks: Vec<TaskWithProject>,
}

pub struct ProjectService;

impl ProjectService {
    pub async fn create_project(
        pool: &SqlitePool,
        user_id: i64,
        dto: CreateProjectDto,
    ) -> Result<ProjectWithStats, AppError> {
        dto.validate()?;

        let uuid = Uuid::new_v4().to_string();
        let color = dto.color.unwrap_or_else(|| "#3b82f6".to_string());
        let status = dto.status.unwrap_or_else(|| "active".to_string()).to_lowercase();

        if !["planning", "active", "on_hold", "completed"].contains(&status.as_str()) {
            return Err(AppError::BadRequest("Invalid project status. Allowed: planning, active, on_hold, completed".to_string()));
        }

        let project_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO projects (uuid, user_id, title, description, color, status, due_date)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#
        )
        .bind(&uuid)
        .bind(user_id)
        .bind(&dto.title)
        .bind(&dto.description)
        .bind(&color)
        .bind(&status)
        .bind(dto.due_date)
        .fetch_one(pool)
        .await?;

        // Log project creation activity
        let _ = sqlx::query(
            "INSERT INTO activity_logs (user_id, action, entity_type, entity_id, details) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(user_id)
        .bind("create_project")
        .bind("project")
        .bind(&uuid)
        .bind(format!("Created project \"{}\"", dto.title))
        .execute(pool)
        .await;

        Self::get_project_by_id(pool, user_id, project_id).await
    }

    pub async fn get_project_by_id(
        pool: &SqlitePool,
        user_id: i64,
        project_id: i64,
    ) -> Result<ProjectWithStats, AppError> {
        let project = sqlx::query_as::<_, ProjectWithStats>(
            r#"
            SELECT p.id, p.uuid, p.user_id, p.title, p.description, p.color, p.status, p.due_date,
                   p.created_at, p.updated_at,
                   COALESCE((SELECT COUNT(*) FROM tasks t WHERE t.project_id = p.id AND t.user_id = p.user_id), 0) as total_tasks,
                   COALESCE((SELECT COUNT(*) FROM tasks t WHERE t.project_id = p.id AND t.user_id = p.user_id AND t.status = 'done'), 0) as completed_tasks
            FROM projects p
            WHERE p.id = ? AND p.user_id = ?
            "#
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found.".to_string()))?;

        Ok(project)
    }

    pub async fn get_project_detail(
        pool: &SqlitePool,
        user_id: i64,
        project_id: i64,
    ) -> Result<ProjectDetailResponse, AppError> {
        let project = Self::get_project_by_id(pool, user_id, project_id).await?;

        let tasks = sqlx::query_as::<_, TaskWithProject>(
            r#"
            SELECT t.id, t.uuid, t.user_id, t.project_id,
                   p.title as project_title, p.color as project_color,
                   t.title, t.description, t.priority, t.status,
                   t.due_date, t.completed_at, t.created_at, t.updated_at
            FROM tasks t
            LEFT JOIN projects p ON p.id = t.project_id
            WHERE t.project_id = ? AND t.user_id = ?
            ORDER BY CASE WHEN t.status = 'done' THEN 1 ELSE 0 END, t.due_date ASC NULLS LAST, t.created_at DESC
            "#
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(ProjectDetailResponse { project, tasks })
    }

    pub async fn list_projects(
        pool: &SqlitePool,
        user_id: i64,
    ) -> Result<Vec<ProjectWithStats>, AppError> {
        let projects = sqlx::query_as::<_, ProjectWithStats>(
            r#"
            SELECT p.id, p.uuid, p.user_id, p.title, p.description, p.color, p.status, p.due_date,
                   p.created_at, p.updated_at,
                   COALESCE((SELECT COUNT(*) FROM tasks t WHERE t.project_id = p.id AND t.user_id = p.user_id), 0) as total_tasks,
                   COALESCE((SELECT COUNT(*) FROM tasks t WHERE t.project_id = p.id AND t.user_id = p.user_id AND t.status = 'done'), 0) as completed_tasks
            FROM projects p
            WHERE p.user_id = ?
            ORDER BY p.updated_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(projects)
    }

    pub async fn update_project(
        pool: &SqlitePool,
        user_id: i64,
        project_id: i64,
        dto: UpdateProjectDto,
    ) -> Result<ProjectWithStats, AppError> {
        dto.validate()?;

        let existing = sqlx::query_as::<_, Project>(
            "SELECT * FROM projects WHERE id = ? AND user_id = ?"
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found.".to_string()))?;

        let title = dto.title.unwrap_or(existing.title);
        let description = dto.description.or(existing.description);
        let color = dto.color.unwrap_or(existing.color);
        let status = dto.status.unwrap_or(existing.status).to_lowercase();
        let due_date = dto.due_date.or(existing.due_date);

        if !["planning", "active", "on_hold", "completed"].contains(&status.as_str()) {
            return Err(AppError::BadRequest("Invalid project status.".to_string()));
        }

        sqlx::query(
            r#"
            UPDATE projects
            SET title = ?, description = ?, color = ?, status = ?, due_date = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ? AND user_id = ?
            "#
        )
        .bind(&title)
        .bind(&description)
        .bind(&color)
        .bind(&status)
        .bind(due_date)
        .bind(project_id)
        .bind(user_id)
        .execute(pool)
        .await?;

        // Log project update activity
        let _ = sqlx::query(
            "INSERT INTO activity_logs (user_id, action, entity_type, entity_id, details) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(user_id)
        .bind("update_project")
        .bind("project")
        .bind(&existing.uuid)
        .bind(format!("Updated project \"{}\"", title))
        .execute(pool)
        .await;

        Self::get_project_by_id(pool, user_id, project_id).await
    }

    pub async fn delete_project(
        pool: &SqlitePool,
        user_id: i64,
        project_id: i64,
    ) -> Result<(), AppError> {
        let existing = sqlx::query_as::<_, Project>(
            "SELECT * FROM projects WHERE id = ? AND user_id = ?"
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found.".to_string()))?;

        // Detach tasks from project first (setting project_id = NULL) to ensure referential safety
        sqlx::query("UPDATE tasks SET project_id = NULL WHERE project_id = ? AND user_id = ?")
            .bind(project_id)
            .bind(user_id)
            .execute(pool)
            .await?;

        // Delete project
        sqlx::query("DELETE FROM projects WHERE id = ? AND user_id = ?")
            .bind(project_id)
            .bind(user_id)
            .execute(pool)
            .await?;

        // Log project deletion activity
        let _ = sqlx::query(
            "INSERT INTO activity_logs (user_id, action, entity_type, entity_id, details) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(user_id)
        .bind("delete_project")
        .bind("project")
        .bind(&existing.uuid)
        .bind(format!("Deleted project \"{}\"", existing.title))
        .execute(pool)
        .await;

        Ok(())
    }
}
