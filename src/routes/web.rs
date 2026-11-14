use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use crate::routes::AppState;

const INDEX_HTML: &str = include_str!("../../templates/index.html");
const LOGIN_HTML: &str = include_str!("../../templates/login.html");
const REGISTER_HTML: &str = include_str!("../../templates/register.html");
const DASHBOARD_HTML: &str = include_str!("../../templates/dashboard.html");
const TASKS_HTML: &str = include_str!("../../templates/tasks.html");
const PROJECTS_HTML: &str = include_str!("../../templates/projects.html");
const CALENDAR_HTML: &str = include_str!("../../templates/calendar.html");
const ANALYTICS_HTML: &str = include_str!("../../templates/analytics.html");
const ADMIN_HTML: &str = include_str!("../../templates/admin.html");

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

async fn render_index() -> impl IntoResponse {
    (StatusCode::OK, Html(INDEX_HTML))
}

async fn render_login() -> impl IntoResponse {
    (StatusCode::OK, Html(LOGIN_HTML))
}

async fn render_register() -> impl IntoResponse {
    (StatusCode::OK, Html(REGISTER_HTML))
}

async fn render_dashboard() -> impl IntoResponse {
    (StatusCode::OK, Html(DASHBOARD_HTML))
}

async fn render_tasks() -> impl IntoResponse {
    (StatusCode::OK, Html(TASKS_HTML))
}

async fn render_projects() -> impl IntoResponse {
    (StatusCode::OK, Html(PROJECTS_HTML))
}

async fn render_calendar() -> impl IntoResponse {
    (StatusCode::OK, Html(CALENDAR_HTML))
}

async fn render_analytics() -> impl IntoResponse {
    (StatusCode::OK, Html(ANALYTICS_HTML))
}

async fn render_admin() -> impl IntoResponse {
    (StatusCode::OK, Html(ADMIN_HTML))
}
