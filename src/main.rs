use std::sync::Arc;
use taskflow::{config, database, routes, services};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "taskflow=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting TASKFLOW application...");

    // Load configuration
    let config = config::Config::from_env().map_err(|e| {
        tracing::error!("Configuration failure: {}", e);
        e
    })?;
    let config = Arc::new(config);

    // Initialize database pool
    let db_pool = database::create_connection_pool(&config.database_url).await?;

    // Run database migrations
    database::run_migrations(&db_pool).await?;

    // Seed default admin account if configured in environment
    services::AuthService::seed_admin(&db_pool, &config).await?;

    // Create shared app state
    let state = routes::AppState {
        db: db_pool,
        config: Arc::clone(&config),
    };

    // Build Axum application router
    let app = routes::create_router(state);

    // Bind TCP listener
    let bind_addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("TASKFLOW server listening on http://{}", bind_addr);

    // Run server with graceful shutdown handling
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("TASKFLOW server stopped cleanly.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C signal, initiating graceful shutdown...");
        },
        _ = terminate => {
            tracing::info!("Received terminate signal, initiating graceful shutdown...");
        },
    }
}
