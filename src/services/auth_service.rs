use argon2::{
    password_hash::{
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};
use chrono::{Duration, Utc};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;
use validator::Validate;

use crate::{
    errors::AppError,
    models::{User, UserResponse},
};

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterDto {
    #[validate(length(min = 3, max = 30, message = "Username must be between 3 and 30 characters"))]
    pub username: String,

    #[validate(email(message = "Invalid email address format"))]
    pub email: String,

    #[validate(length(min = 8, max = 128, message = "Password must be between 8 and 128 characters"))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginDto {
    #[validate(length(min = 1, message = "Username or email is required"))]
    pub username_or_email: String,

    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub token: String,
    pub expires_at: chrono::DateTime<Utc>,
}

pub struct AuthService;

impl AuthService {
    pub fn hash_password(password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| AppError::InternalServerError(format!("Password hashing failed: {}", e)))
    }

    pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
        let parsed_hash = PasswordHash::new(password_hash)
            .map_err(|e| AppError::InternalServerError(format!("Invalid password hash: {}", e)))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    pub async fn register(pool: &SqlitePool, dto: RegisterDto) -> Result<UserResponse, AppError> {
        dto.validate()?;

        let username = dto.username.trim().to_lowercase();
        let email = dto.email.trim().to_lowercase();

        // Check if username or email already exists
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM users WHERE username = ? OR email = ? LIMIT 1"
        )
        .bind(&username)
        .bind(&email)
        .fetch_optional(pool)
        .await?;

        if existing.is_some() {
            return Err(AppError::Conflict(
                "A user with that username or email already exists.".to_string(),
            ));
        }

        // First registered user automatically becomes admin
        let user_count: (i64,) = sqlx::query_as("SELECT count(*) FROM users")
            .fetch_one(pool)
            .await?;
        let role = if user_count.0 == 0 { "admin" } else { "user" };

        let password_hash = Self::hash_password(&dto.password)?;
        let user_uuid = Uuid::new_v4().to_string();

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (uuid, username, email, password_hash, role, is_active)
            VALUES (?, ?, ?, ?, ?, 1)
            RETURNING id, uuid, username, email, password_hash, role, is_active, created_at, updated_at
            "#
        )
        .bind(&user_uuid)
        .bind(&username)
        .bind(&email)
        .bind(&password_hash)
        .bind(role)
        .fetch_one(pool)
        .await?;

        // Log registration activity
        let _ = sqlx::query(
            "INSERT INTO activity_logs (user_id, action, entity_type, entity_id, details) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(user.id)
        .bind("register")
        .bind("user")
        .bind(&user.uuid)
        .bind("User registered successfully")
        .execute(pool)
        .await;

        Ok(UserResponse::from(user))
    }

    pub async fn login(
        pool: &SqlitePool,
        dto: LoginDto,
        session_max_age_secs: u64,
        user_agent: Option<String>,
    ) -> Result<AuthResponse, AppError> {
        dto.validate()?;

        let identifier = dto.username_or_email.trim().to_lowercase();

        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, uuid, username, email, password_hash, role, is_active, created_at, updated_at
            FROM users
            WHERE username = ? OR email = ?
            LIMIT 1
            "#
        )
        .bind(&identifier)
        .bind(&identifier)
        .fetch_optional(pool)
        .await?;

        let user = match user {
            Some(u) => u,
            None => {
                // Execute a dummy password verify to prevent timing-based user enumeration
                let dummy_hash = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzdHJpbmc$bW9ja2hhc2h2YWx1ZWZvcnRpbWluZw";
                let _ = Self::verify_password(&dto.password, dummy_hash);
                return Err(AppError::Unauthorized("Invalid username/email or password.".to_string()));
            }
        };

        if !user.is_active {
            return Err(AppError::Forbidden("Your account has been deactivated. Please contact support.".to_string()));
        }

        let is_valid = Self::verify_password(&dto.password, &user.password_hash)?;
        if !is_valid {
            return Err(AppError::Unauthorized("Invalid username/email or password.".to_string()));
        }

        // Create new session
        let token = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::seconds(session_max_age_secs as i64);

        sqlx::query(
            r#"
            INSERT INTO sessions (token, user_id, expires_at, user_agent)
            VALUES (?, ?, ?, ?)
            "#
        )
        .bind(&token)
        .bind(user.id)
        .bind(expires_at)
        .bind(user_agent)
        .execute(pool)
        .await?;

        // Log login activity
        let _ = sqlx::query(
            "INSERT INTO activity_logs (user_id, action, entity_type, details) VALUES (?, ?, ?, ?)"
        )
        .bind(user.id)
        .bind("login")
        .bind("session")
        .bind("User logged in")
        .execute(pool)
        .await;

        Ok(AuthResponse {
            user: UserResponse::from(user),
            token,
            expires_at,
        })
    }

    pub async fn logout(pool: &SqlitePool, token: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(token)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_user_by_session(pool: &SqlitePool, token: &str) -> Result<UserResponse, AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT u.id, u.uuid, u.username, u.email, u.password_hash, u.role, u.is_active, u.created_at, u.updated_at
            FROM users u
            INNER JOIN sessions s ON s.user_id = u.id
            WHERE s.token = ? AND datetime(s.expires_at) > datetime('now') AND u.is_active = 1
            LIMIT 1
            "#
        )
        .bind(token)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid or expired session.".to_string()))?;

        Ok(UserResponse::from(user))
    }
}
