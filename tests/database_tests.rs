use sqlx::Row;
use taskflow::database::{create_connection_pool, run_migrations};
use uuid::Uuid;

#[tokio::test]
async fn test_sqlite_migrations_and_schema() {
    let pool = create_connection_pool("sqlite::memory:")
        .await
        .expect("Failed to initialize in-memory DB");

    run_migrations(&pool)
        .await
        .expect("Failed to run schema migrations");

    // Verify tables exist
    let row = sqlx::query(
        "SELECT count(*) as cnt FROM sqlite_master WHERE type='table' AND name IN ('users', 'sessions', 'projects', 'tasks', 'activity_logs')"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to query sqlite_master");

    let count: i64 = row.get("cnt");
    assert_eq!(count, 5, "Expected all 5 core tables to be created");
}

#[tokio::test]
async fn test_user_creation_and_constraints() {
    let pool = create_connection_pool("sqlite::memory:")
        .await
        .expect("Failed to initialize in-memory DB");
    run_migrations(&pool).await.expect("Migrations failed");

    let user_uuid = Uuid::new_v4().to_string();

    // Insert user
    let res = sqlx::query(
        "INSERT INTO users (uuid, username, email, password_hash, role) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&user_uuid)
    .bind("johndoe")
    .bind("john@example.com")
    .bind("hashed_pass_placeholder")
    .bind("user")
    .execute(&pool)
    .await;

    assert!(res.is_ok(), "User insertion should succeed");

    // Test UNIQUE constraint on username (case-insensitive due to NOCASE)
    let dup_username = sqlx::query(
        "INSERT INTO users (uuid, username, email, password_hash, role) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(Uuid::new_v4().to_string())
    .bind("JohnDoe") // same name, different case
    .bind("other@example.com")
    .bind("pass")
    .bind("user")
    .execute(&pool)
    .await;

    assert!(dup_username.is_err(), "Duplicate username should fail UNIQUE constraint");

    // Test UNIQUE constraint on email
    let dup_email = sqlx::query(
        "INSERT INTO users (uuid, username, email, password_hash, role) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(Uuid::new_v4().to_string())
    .bind("otheruser")
    .bind("JOHN@example.com") // same email, different case
    .bind("pass")
    .bind("user")
    .execute(&pool)
    .await;

    assert!(dup_email.is_err(), "Duplicate email should fail UNIQUE constraint");
}

#[tokio::test]
async fn test_cascading_deletes() {
    let pool = create_connection_pool("sqlite::memory:")
        .await
        .expect("Failed to initialize in-memory DB");
    run_migrations(&pool).await.expect("Migrations failed");

    // 1. Create a user
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (uuid, username, email, password_hash) VALUES (?, ?, ?, ?) RETURNING id"
    )
    .bind(Uuid::new_v4().to_string())
    .bind("alice")
    .bind("alice@example.com")
    .bind("hash")
    .fetch_one(&pool)
    .await
    .unwrap();

    // 2. Create a project for user
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (uuid, user_id, title) VALUES (?, ?, ?) RETURNING id"
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind("Website Redesign")
    .fetch_one(&pool)
    .await
    .unwrap();

    // 3. Create a task for user and project
    sqlx::query(
        "INSERT INTO tasks (uuid, user_id, project_id, title, priority, status) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(project_id)
    .bind("Design wireframes")
    .bind("high")
    .bind("in_progress")
    .execute(&pool)
    .await
    .unwrap();

    // 4. Verify task count
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM tasks WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);

    // 5. Delete the user
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    // 6. Verify cascading delete took effect
    let remaining_tasks: i64 = sqlx::query_scalar("SELECT count(*) FROM tasks WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining_tasks, 0, "Tasks should be deleted when owner user is deleted");

    let remaining_projects: i64 = sqlx::query_scalar("SELECT count(*) FROM projects WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining_projects, 0, "Projects should be deleted when owner user is deleted");
}
