use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    User,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::User => write!(f, "user"),
        }
    }
}

impl From<String> for UserRole {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "admin" => UserRole::Admin,
            _ => UserRole::User,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub uuid: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: i64,
    pub uuid: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            uuid: u.uuid,
            username: u.username,
            email: u.email,
            role: u.role,
            is_active: u.is_active,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}
