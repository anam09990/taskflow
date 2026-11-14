use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::errors::AppError;

#[derive(Debug, Deserialize)]
pub struct CalendarQuery {
    pub year: Option<i32>,
    pub month: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct CalendarEvent {
    pub id: i64,
    pub title: String,
    pub event_type: String, // "task" | "project"
    pub due_date: DateTime<Utc>,
    pub priority: Option<String>,
    pub status: String,
    pub project_id: Option<i64>,
    pub project_title: Option<String>,
    pub project_color: Option<String>,
}

pub struct CalendarService;

impl CalendarService {
    pub async fn get_calendar_events(
        pool: &SqlitePool,
        user_id: i64,
        query: CalendarQuery,
    ) -> Result<Vec<CalendarEvent>, AppError> {
        let now = Utc::now();
        let year = query.year.unwrap_or_else(|| now.year());
        let month = query.month.unwrap_or_else(|| now.month());

        if !(1..=12).contains(&month) {
            return Err(AppError::BadRequest("Month must be between 1 and 12.".to_string()));
        }

        let start_date = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| AppError::BadRequest("Invalid calendar date parameters.".to_string()))?
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let (next_year, next_month) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };

        let end_date = NaiveDate::from_ymd_opt(next_year, next_month, 1)
            .ok_or_else(|| AppError::BadRequest("Invalid calendar end-date.".to_string()))?
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let start_utc = Utc.from_utc_datetime(&start_date);
        let end_utc = Utc.from_utc_datetime(&end_date);

        // Fetch tasks with due dates in this range
        let tasks = sqlx::query_as::<_, (i64, String, String, String, DateTime<Utc>, Option<i64>, Option<String>, Option<String>)>(
            r#"
            SELECT t.id, t.title, t.priority, t.status, t.due_date,
                   p.id as project_id, p.title as project_title, p.color as project_color
            FROM tasks t
            LEFT JOIN projects p ON p.id = t.project_id
            WHERE t.user_id = ? AND t.due_date >= ? AND t.due_date < ?
            ORDER BY t.due_date ASC
            "#
        )
        .bind(user_id)
        .bind(start_utc)
        .bind(end_utc)
        .fetch_all(pool)
        .await?;

        let mut events: Vec<CalendarEvent> = tasks
            .into_iter()
            .map(|row| CalendarEvent {
                id: row.0,
                title: row.1,
                event_type: "task".to_string(),
                priority: Some(row.2),
                status: row.3,
                due_date: row.4,
                project_id: row.5,
                project_title: row.6,
                project_color: row.7,
            })
            .collect();

        // Fetch projects with due dates in this range
        let projects = sqlx::query_as::<_, (i64, String, String, String, DateTime<Utc>)>(
            r#"
            SELECT id, title, color, status, due_date
            FROM projects
            WHERE user_id = ? AND due_date >= ? AND due_date < ?
            ORDER BY due_date ASC
            "#
        )
        .bind(user_id)
        .bind(start_utc)
        .bind(end_utc)
        .fetch_all(pool)
        .await?;

        for row in projects {
            events.push(CalendarEvent {
                id: row.0,
                title: format!("Project Deadline: {}", row.1),
                event_type: "project".to_string(),
                priority: Some("high".to_string()),
                status: row.3,
                due_date: row.4,
                project_id: Some(row.0),
                project_title: Some(row.1),
                project_color: Some(row.2),
            });
        }

        // Sort combined events by due_date
        events.sort_by(|a, b| a.due_date.cmp(&b.due_date));

        Ok(events)
    }
}
