use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Planning,
    Active,
    OnHold,
    Completed,
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectStatus::Planning => write!(f, "planning"),
            ProjectStatus::Active => write!(f, "active"),
            ProjectStatus::OnHold => write!(f, "on_hold"),
            ProjectStatus::Completed => write!(f, "completed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: i64,
    pub uuid: String,
    pub user_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub color: String,
    pub status: String,
    pub due_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectWithStats {
    pub id: i64,
    pub uuid: String,
    pub user_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub color: String,
    pub status: String,
    pub due_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_tasks: i64,
    pub completed_tasks: i64,
}
