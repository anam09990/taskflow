use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActivityLog {
    pub id: i64,
    pub user_id: i64,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub details: Option<String>,
    pub created_at: DateTime<Utc>,
}
