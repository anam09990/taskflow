use std::collections::HashMap;
use chrono::{Duration, Utc};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::{
    errors::AppError,
    models::{ActivityLog, TaskWithProject},
};

#[derive(Debug, Serialize)]
pub struct DashboardOverview {
    pub total_tasks: i64,
    pub completed_tasks: i64,
    pub pending_tasks: i64,
    pub completion_rate: f64,
    pub upcoming_deadlines: Vec<TaskWithProject>,
    pub recent_activity: Vec<ActivityLog>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DailyVelocity {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct DistributionItem {
    pub label: String,
    pub count: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProjectWorkloadItem {
    pub project_id: i64,
    pub project_title: String,
    pub color: String,
    pub task_count: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct ProductivityReport {
    pub total_tasks: i64,
    pub completed_tasks: i64,
    pub pending_tasks: i64,
    pub overdue_tasks: i64,
    pub completion_rate: f64,
    pub weekly_velocity: Vec<DailyVelocity>,
    pub monthly_velocity: Vec<DailyVelocity>,
    pub priority_distribution: Vec<DistributionItem>,
    pub status_distribution: Vec<DistributionItem>,
    pub project_distribution: Vec<ProjectWorkloadItem>,
    pub weekly_completed_count: i64,
    pub monthly_completed_count: i64,
}

pub struct AnalyticsService;

impl AnalyticsService {
    pub async fn get_dashboard_overview(
        pool: &SqlitePool,
        user_id: i64,
    ) -> Result<DashboardOverview, AppError> {
        let total_tasks: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tasks WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        let completed_tasks: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tasks WHERE user_id = ? AND status = 'done'"
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        let pending_tasks = total_tasks - completed_tasks;

        let completion_rate = if total_tasks > 0 {
            ((completed_tasks as f64 / total_tasks as f64) * 100.0).round()
        } else {
            0.0
        };

        let upcoming_deadlines = sqlx::query_as::<_, TaskWithProject>(
            r#"
            SELECT t.id, t.uuid, t.user_id, t.project_id,
                   p.title as project_title, p.color as project_color,
                   t.title, t.description, t.priority, t.status,
                   t.due_date, t.completed_at, t.created_at, t.updated_at
            FROM tasks t
            LEFT JOIN projects p ON p.id = t.project_id
            WHERE t.user_id = ? AND t.status != 'done' AND t.due_date IS NOT NULL
            ORDER BY t.due_date ASC
            LIMIT 5
            "#
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        let recent_activity = sqlx::query_as::<_, ActivityLog>(
            r#"
            SELECT id, user_id, action, entity_type, entity_id, details, created_at
            FROM activity_logs
            WHERE user_id = ?
            ORDER BY created_at DESC
            LIMIT 6
            "#
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(DashboardOverview {
            total_tasks,
            completed_tasks,
            pending_tasks,
            completion_rate,
            upcoming_deadlines,
            recent_activity,
        })
    }

    pub async fn get_productivity_report(
        pool: &SqlitePool,
        user_id: i64,
    ) -> Result<ProductivityReport, AppError> {
        // Overall task counts
        let total_tasks: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tasks WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        let completed_tasks: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tasks WHERE user_id = ? AND status = 'done'"
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        let pending_tasks = total_tasks - completed_tasks;

        let completion_rate = if total_tasks > 0 {
            ((completed_tasks as f64 / total_tasks as f64) * 100.0).round()
        } else {
            0.0
        };

        // Overdue tasks (due_date in past, status not done)
        let overdue_tasks: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tasks WHERE user_id = ? AND status != 'done' AND datetime(due_date) < datetime('now')"
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        // Fetch daily completed task counts for the last 30 days in a single grouped query
        let daily_rows = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT date(completed_at) as day_date, count(*) as count
            FROM tasks
            WHERE user_id = ? AND status = 'done' AND completed_at IS NOT NULL
              AND date(completed_at) >= date('now', '-30 days')
            GROUP BY date(completed_at)
            "#
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        let daily_counts: HashMap<String, i64> = daily_rows.into_iter().collect();

        // 7-day velocity series
        let now = Utc::now();
        let mut weekly_velocity = Vec::with_capacity(7);
        let mut weekly_completed_count = 0;

        for i in (0..7).rev() {
            let day = now - Duration::days(i);
            let day_str = day.format("%Y-%m-%d").to_string();
            let count = daily_counts.get(&day_str).copied().unwrap_or(0);

            weekly_completed_count += count;
            weekly_velocity.push(DailyVelocity {
                date: day.format("%b %d").to_string(),
                count,
            });
        }

        // 30-day velocity series
        let mut monthly_velocity = Vec::with_capacity(30);
        let mut monthly_completed_count = 0;

        for i in (0..30).rev() {
            let day = now - Duration::days(i);
            let day_str = day.format("%Y-%m-%d").to_string();
            let count = daily_counts.get(&day_str).copied().unwrap_or(0);

            monthly_completed_count += count;
            monthly_velocity.push(DailyVelocity {
                date: day.format("%b %d").to_string(),
                count,
            });
        }

        // Priority Distribution
        let priorities = ["urgent", "high", "medium", "low"];
        let mut priority_distribution = Vec::new();
        for p in &priorities {
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM tasks WHERE user_id = ? AND priority = ?"
            )
            .bind(user_id)
            .bind(p)
            .fetch_one(pool)
            .await?;

            let percentage = if total_tasks > 0 {
                ((count as f64 / total_tasks as f64) * 100.0).round()
            } else {
                0.0
            };

            priority_distribution.push(DistributionItem {
                label: p.to_string(),
                count,
                percentage,
            });
        }

        // Status Distribution
        let statuses = ["todo", "in_progress", "review", "done"];
        let mut status_distribution = Vec::new();
        for s in &statuses {
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM tasks WHERE user_id = ? AND status = ?"
            )
            .bind(user_id)
            .bind(s)
            .fetch_one(pool)
            .await?;

            let percentage = if total_tasks > 0 {
                ((count as f64 / total_tasks as f64) * 100.0).round()
            } else {
                0.0
            };

            status_distribution.push(DistributionItem {
                label: s.to_string(),
                count,
                percentage,
            });
        }

        // Project workload distribution
        let project_rows = sqlx::query_as::<_, (i64, String, String, i64)>(
            r#"
            SELECT p.id, p.title, p.color, count(t.id) as task_count
            FROM projects p
            LEFT JOIN tasks t ON t.project_id = p.id AND t.user_id = p.user_id
            WHERE p.user_id = ?
            GROUP BY p.id
            ORDER BY task_count DESC
            "#
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        let mut project_distribution = Vec::new();
        for (pid, title, color, count) in project_rows {
            let percentage = if total_tasks > 0 {
                ((count as f64 / total_tasks as f64) * 100.0).round()
            } else {
                0.0
            };

            project_distribution.push(ProjectWorkloadItem {
                project_id: pid,
                project_title: title,
                color,
                task_count: count,
                percentage,
            });
        }

        Ok(ProductivityReport {
            total_tasks,
            completed_tasks,
            pending_tasks,
            overdue_tasks,
            completion_rate,
            weekly_velocity,
            monthly_velocity,
            priority_distribution,
            status_distribution,
            project_distribution,
            weekly_completed_count,
            monthly_completed_count,
        })
    }
}
