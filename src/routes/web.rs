use axum::{
    response::{Html, IntoResponse},
    routing::get,
    http::StatusCode,
    Router,
};
use crate::routes::AppState;

pub fn web_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(render_index))
        .route("/login", get(render_login))
        .route("/register", get(render_register))
        .route("/dashboard", get(render_dashboard))
        .route("/tasks", get(render_tasks))
        .route("/projects", get(render_projects))
        .route("/calendar", get(render_calendar))
        .route("/analytics", get(render_analytics))
        .route("/admin", get(render_admin))
}

async fn serve_template(path: &str) -> impl IntoResponse {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => (StatusCode::OK, Html(content)).into_response(),
        Err(err) => {
            tracing::warn!("Failed to read template at {}: {}", path, err);
            (
                StatusCode::NOT_FOUND,
                Html("<h1>404 - Page Not Found</h1>".to_string()),
            )
                .into_response()
        }
    }
}

async fn render_index() -> impl IntoResponse {
    serve_template("templates/index.html").await
}

async fn render_login() -> impl IntoResponse {
    serve_template("templates/login.html").await
}

async fn render_register() -> impl IntoResponse {
    serve_template("templates/register.html").await
}

async fn render_dashboard() -> impl IntoResponse {
    serve_template("templates/dashboard.html").await
}

async fn render_tasks() -> impl IntoResponse {
    serve_template("templates/tasks.html").await
}

async fn render_projects() -> impl IntoResponse {
    serve_template("templates/projects.html").await
}

async fn render_calendar() -> impl IntoResponse {
    serve_template("templates/calendar.html").await
}

async fn render_analytics() -> impl IntoResponse {
    serve_template("templates/analytics.html").await
}

async fn render_admin() -> impl IntoResponse {
    serve_template("templates/admin.html").await
}
