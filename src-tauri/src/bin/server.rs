//! StockenBoard — Standalone web server binary (no Tauri dependencies).
//!
//! Configuration via environment variables:
//! - `SB_BIND`       — Network interface bind address (default: `0.0.0.0`)
//! - `SB_PORT`       — HTTP server port (default: `8080`)
//! - `SB_DATA_DIR`   — Path to persistent data directory (default: `./data`)
//! - `SB_STATIC_DIR` — Path to built SPA static files (default: `./static`)

use std::sync::Arc;

use stockenboard_lib::config::ServerConfig;
use stockenboard_lib::core_state::CoreState;
use stockenboard_lib::api;

#[tokio::main]
async fn main() {
    // ─── Parse configuration from environment ───────────────────────────────────
    let config = ServerConfig::from_env().unwrap_or_else(|e| {
        eprintln!("[Server] {}", e);
        std::process::exit(1);
    });
    let bind = config.bind;
    let port = config.port;
    let data_dir = config.data_dir;

    // ─── Create data directory if it does not exist ─────────────────────────────
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!(
            "[Server] Failed to create data directory '{}': {}",
            data_dir.display(),
            e
        );
        std::process::exit(1);
    }

    // ─── Initialize shared core state ───────────────────────────────────────────
    let state = CoreState::new(&data_dir).unwrap_or_else(|e| {
        eprintln!("[Server] Failed to initialize core state: {}", e);
        std::process::exit(1);
    });

    // Start background tasks (Notification Engine + AI Scheduler)
    state.start_background_tasks().await;

    // Auto-set unattended based on active recordings at startup
    let active_count = state.db.count_active_recordings().unwrap_or(0);
    if active_count > 0 {
        state.polling.set_unattended(true).await;
    }

    // Start polling
    state.polling.start(
        state.db.clone(),
        state.registry.clone(),
        state.event_bus.clone(),
    );

    // ─── Build Axum router (with static file serving for SPA) ─────────────────
    let static_dir = std::env::var("SB_STATIC_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("./static"));
    let app = api::build_router_with_static(Arc::new(state), &static_dir);

    // ─── Bind TCP listener and start serving ────────────────────────────────────
    let addr = format!("{}:{}", bind, port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|e| {
        eprintln!("[Server] Failed to bind to {}: {}", addr, e);
        std::process::exit(1);
    });

    println!("[Server] Listening on http://{}:{}", bind, port);

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("[Server] Unexpected server error");
}

/// Waits for a shutdown signal (Ctrl+C / SIGTERM).
async fn shutdown_signal() {
    // ctrl_c handles SIGINT on all platforms
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to register SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {},
            _ = sigterm.recv() => {},
        }
    }

    #[cfg(not(unix))]
    {
        let _ = ctrl_c.await;
    }

    println!("[Server] Shutdown signal received, stopping gracefully...");
}
