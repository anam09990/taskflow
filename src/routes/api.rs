use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use crate::{
    handlers::{
        admin_handler, analytics_handler, auth_handler, calendar_handler, health_check,
        project_handler, task_handler,
    },
    routes::AppState,
};

pub fn api_routes() -> Router<AppState> {
    let auth_router = Router::new()
        .route("/register", post(auth_handler::register))
        .route("/login", post(auth_handler::login))
        .route("/logout", post(auth_handler::logout))
        .route("/me", get(auth_handler::me));

    let analytics_router = Router::new()
        .route("/overview", get(analytics_handler::get_dashboard_overview))
        .route("/report", get(analytics_handler::get_productivity_report));

    let task_router = Router::new()
        .route("/", post(task_handler::create_task))
        .route("/", get(task_handler::list_tasks))
        .route("/{id}", get(task_handler::get_task))
        .route("/{id}", put(task_handler::update_task))
        .route("/{id}/status", patch(task_handler::update_task_status))
        .route("/{id}", delete(task_handler::delete_task));

    let project_router = Router::new()
        .route("/", post(project_handler::create_project))
        .route("/", get(project_handler::list_projects))
        .route("/{id}", get(project_handler::get_project))
        .route("/{id}/detail", get(project_handler::get_project_detail))
        .route("/{id}", put(project_handler::update_project))
        .route("/{id}", delete(project_handler::delete_project));

    let calendar_router = Router::new()
        .route("/events", get(calendar_handler::get_calendar_events));

    let admin_router = Router::new()
        .route("/stats", get(admin_handler::get_platform_stats))
        .route("/users", get(admin_handler::list_users))
        .route("/users/{id}/status", patch(admin_handler::update_user_status))
        .route("/users/{id}/role", patch(admin_handler::update_user_role));

    Router::new()
        .route("/health", get(health_check))
        .nest("/auth", auth_router)
        .nest("/analytics", analytics_router)
        .nest("/tasks", task_router)
        .nest("/projects", project_router)
        .nest("/calendar", calendar_router)
        .nest("/admin", admin_router)
}
