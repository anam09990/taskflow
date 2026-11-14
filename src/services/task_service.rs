use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;
use validator::Validate;

use crate::{
    errors::AppError,
    models::{Task, TaskWithProject},
};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTaskDto {
    #[validate(length(min = 1, max = 255, message = "Task title must be between 1 and 255 characters"))]
    pub title: String,

    pub description: Option<String>,
    pub project_id: Option<i64>,

    pub priority: Option<String>,
    pub status: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTaskDto {
    #[validate(length(min = 1, max = 255, message = "Task title must be between 1 and 255 characters"))]
    pub title: Option<String>,

    pub description: Option<String>,
    pub project_id: Option<i64>,
    pub priority: Option<String>,
    pub status: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskStatusDto {
    pub status: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct TaskFilterQuery {
    pub search: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub project_id: Option<i64>,
}

pub struct TaskService;

impl TaskService {
    pub async fn create_task(
        pool: &SqlitePool,
        user_id: i64,
        dto: CreateTaskDto,
    ) -> Result<TaskWithProject, AppError> {
        dto.validate()?;

        let uuid = Uuid::new_v4().to_string();
        let priority = dto.priority.unwrap_or_else(|| "medium".to_string()).to_lowercase();
        let status = dto.status.unwrap_or_else(|| "todo".to_string()).to_lowercase();

        // Validate priority enum
        if !["low", "medium", "high", "urgent"].contains(&priority.as_str()) {
            return Err(AppError::BadRequest("Invalid priority. Allowed: low, medium, high, urgent".to_string()));
        }

        // Validate status enum
        if !["todo", "in_progress", "review", "done"].contains(&status.as_str()) {
            return Err(AppError::BadRequest("Invalid status. Allowed: todo, in_progress, review, done".to_string()));
        }

        // If project_id provided, ensure project belongs to user
        if let Some(pid) = dto.project_id {
            let project_exists: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM projects WHERE id = ? AND user_id = ?"
            )
            .bind(pid)
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

            if project_exists.is_none() {
                return Err(AppError::BadRequest("Specified project does not exist or does not belong to you.".to_string()));
            }
        }

        let completed_at = if status == "done" { Some(Utc::now()) } else { None };

        let task_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO tasks (uuid, user_id, project_id, title, description, priority, status, due_date, completed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#
        )
        .bind(&uuid)
        .bind(user_id)
        .bind(dto.project_id)
        .bind(&dto.title)
        .bind(&dto.description)
        .bind(&priority)
        .bind(&status)
        .bind(dto.due_date)
        .bind(completed_at)
        .fetch_one(pool)
        .await?;

        // Log task creation activity
        let _ = sqlx::query(
            "INSERT INTO activity_logs (user_id, action, entity_type, entity_id, details) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(user_id)
        .bind("create_task")
        .bind("task")
        .bind(&uuid)
        .bind(format!("Created task \"{}\"", dto.title))
        .execute(pool)
        .await;

        Self::get_task_by_id(pool, user_id, task_id).await
    }

    pub async fn get_task_by_id(
        pool: &SqlitePool,
        user_id: i64,
        task_id: i64,
    ) -> Result<TaskWithProject, AppError> {
        let task = sqlx::query_as::<_, TaskWithProject>(
            r#"
            SELECT t.id, t.uuid, t.user_id, t.project_id,
                   p.title as project_title, p.color as project_color,
                   t.title, t.description, t.priority, t.status,
                   t.due_date, t.completed_at, t.created_at, t.updated_at
            FROM tasks t
            LEFT JOIN projects p ON p.id = t.project_id
            WHERE t.id = ? AND t.user_id = ?
            "#
        )
        .bind(task_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Task not found.".to_string()))?;

        Ok(task)
    }

    pub async fn list_tasks(
        pool: &SqlitePool,
        user_id: i64,
        filter: TaskFilterQuery,
    ) -> Result<Vec<TaskWithProject>, AppError> {
        let mut query = String::from(
            r#"
            SELECT t.id, t.uuid, t.user_id, t.project_id,
                   p.title as project_title, p.color as project_color,
                   t.title, t.description, t.priority, t.status,
                   t.due_date, t.completed_at, t.created_at, t.updated_at
            FROM tasks t
            LEFT JOIN projects p ON p.id = t.project_id
            WHERE t.user_id = ?
            "#
        );

        if let Some(status) = &filter.status {
            if !status.is_empty() && status != "all" {
                query.push_str(" AND t.status = ?");
            }
        }

        if let Some(priority) = &filter.priority {
            if !priority.is_empty() && priority != "all" {
                query.push_str(" AND t.priority = ?");
            }
        }

        if let Some(_pid) = filter.project_id {
            query.push_str(" AND t.project_id = ?");
        }

        if let Some(search) = &filter.search {
            if !search.trim().is_empty() {
                query.push_str(" AND (t.title LIKE ? OR t.description LIKE ?)");
            }
        }

        query.push_str(" ORDER BY CASE WHEN t.status = 'done' THEN 1 ELSE 0 END, t.due_date ASC NULLS LAST, t.created_at DESC");

        let mut sqlx_query = sqlx::query_as::<_, TaskWithProject>(&query).bind(user_id);

        if let Some(status) = &filter.status {
            if !status.is_empty() && status != "all" {
                sqlx_query = sqlx_query.bind(status);
            }
        }

        if let Some(priority) = &filter.priority {
            if !priority.is_empty() && priority != "all" {
                sqlx_query = sqlx_query.bind(priority);
            }
        }

        if let Some(pid) = filter.project_id {
            sqlx_query = sqlx_query.bind(pid);
        }

        if let Some(search) = &filter.search {
            if !search.trim().is_empty() {
                let search_param = format!("%{}%", search.trim());
                sqlx_query = sqlx_query.bind(search_param.clone()).bind(search_param);
            }
        }

        let tasks = sqlx_query.fetch_all(pool).await?;
        Ok(tasks)
    }

    pub async fn update_task(
        pool: &SqlitePool,
        user_id: i64,
        task_id: i64,
        dto: UpdateTaskDto,
    ) -> Result<TaskWithProject, AppError> {
        dto.validate()?;

        // Ensure task exists and belongs to user
        let existing = sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks WHERE id = ? AND user_id = ?"
        )
        .bind(task_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Task not found.".to_string()))?;

        let title = dto.title.unwrap_or(existing.title);
        let description = dto.description.or(existing.description);
        let project_id = dto.project_id.or(existing.project_id);
        let priority = dto.priority.unwrap_or(existing.priority).to_lowercase();
        let status = dto.status.unwrap_or(existing.status).to_lowercase();
        let due_date = dto.due_date.or(existing.due_date);

        if !["low", "medium", "high", "urgent"].contains(&priority.as_str()) {
            return Err(AppError::BadRequest("Invalid priority.".to_string()));
        }

        if !["todo", "in_progress", "review", "done"].contains(&status.as_str()) {
            return Err(AppError::BadRequest("Invalid status.".to_string()));
        }

        if let Some(pid) = project_id {
            let project_exists: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM projects WHERE id = ? AND user_id = ?"
            )
            .bind(pid)
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

            if project_exists.is_none() {
                return Err(AppError::BadRequest("Specified project does not exist.".to_string()));
            }
        }

        let completed_at = if status == "done" {
            if existing.completed_at.is_none() { Some(Utc::now()) } else { existing.completed_at }
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE tasks
            SET title = ?, description = ?, project_id = ?, priority = ?, status = ?,
                due_date = ?, completed_at = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ? AND user_id = ?
            "#
        )
        .bind(&title)
        .bind(&description)
        .bind(project_id)
        .bind(&priority)
        .bind(&status)
        .bind(due_date)
        .bind(completed_at)
        .bind(task_id)
        .bind(user_id)
        .execute(pool)
        .await?;

        // Log task update activity
        let _ = sqlx::query(
            "INSERT INTO activity_logs (user_id, action, entity_type, entity_id, details) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(user_id)
        .bind("update_task")
        .bind("task")
        .bind(&existing.uuid)
        .bind(format!("Updated task \"{}\"", title))
        .execute(pool)
        .await;

        Self::get_task_by_id(pool, user_id, task_id).await
    }

    pub async fn update_task_status(
        pool: &SqlitePool,
        user_id: i64,
        task_id: i64,
        status: String,
    ) -> Result<TaskWithProject, AppError> {
        let status = status.to_lowercase();
        if !["todo", "in_progress", "review", "done"].contains(&status.as_str()) {
            return Err(AppError::BadRequest("Invalid status value.".to_string()));
        }

        let existing = sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks WHERE id = ? AND user_id = ?"
        )
        .bind(task_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Task not found.".to_string()))?;

        let completed_at = if status == "done" { Some(Utc::now()) } else { None };

        sqlx::query(
            r#"
            UPDATE tasks
            SET status = ?, completed_at = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ? AND user_id = ?
            "#
        )
        .bind(&status)
        .bind(completed_at)
        .bind(task_id)
        .bind(user_id)
        .execute(pool)
        .await?;

        // Log status change activity
        let _ = sqlx::query(
            "INSERT INTO activity_logs (user_id, action, entity_type, entity_id, details) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(user_id)
        .bind("status_change")
        .bind("task")
        .bind(&existing.uuid)
        .bind(format!("Changed task \"{}\" status to {}", existing.title, status))
        .execute(pool)
        .await;

        Self::get_task_by_id(pool, user_id, task_id).await
    }

    pub async fn delete_task(
        pool: &SqlitePool,
        user_id: i64,
        task_id: i64,
    ) -> Result<(), AppError> {
        let existing = sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks WHERE id = ? AND user_id = ?"
        )
        .bind(task_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Task not found.".to_string()))?;

        sqlx::query("DELETE FROM tasks WHERE id = ? AND user_id = ?")
            .bind(task_id)
            .bind(user_id)
            .execute(pool)
            .await?;

        // Log task deletion activity
        let _ = sqlx::query(
            "INSERT INTO activity_logs (user_id, action, entity_type, entity_id, details) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(user_id)
        .bind("delete_task")
        .bind("task")
        .bind(&existing.uuid)
        .bind(format!("Deleted task \"{}\"", existing.title))
        .execute(pool)
        .await;

        Ok(())
    }
}
