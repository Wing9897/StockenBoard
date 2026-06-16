//! System, icon, data, and DEX endpoints.
//!
//! Provides:
//! - `GET /system/config` — get system config (api_port, unattended_polling)
//! - `PUT /system/config` — set system config
//! - `POST /system/reload-polling` — reload polling
//! - `POST /system/reset` — reset all data
//! - `GET /system/data-dir` — get data directory path
//! - `POST /icons/:symbol` — set icon (raw bytes upload)
//! - `DELETE /icons/:symbol` — remove icon
//! - `GET /icons` — list available icons
//! - `POST /icons/download-logos` — download logos for all subscriptions
//! - `GET /data/export` — export data
//! - `POST /data/import` — import data
//! - `GET /dex/pool/:provider/:address` — lookup DEX pool

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Json, Path, Query, State},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResponse};
use crate::core_state::CoreState;
use crate::db::ExportData;
use crate::providers::create_dex_lookup;

// ─── Request / Response Types ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct SystemConfig {
    api_port: u16,
    unattended_polling: bool,
    api_enabled: bool,
}

#[derive(Debug, Deserialize)]
struct SetSystemConfig {
    api_port: Option<u16>,
    unattended_polling: Option<bool>,
    api_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SetVisibleSubscriptionsRequest {
    ids: Vec<i64>,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReadFileQuery {
    path: String,
}

#[derive(Debug, Deserialize)]
struct DesktopOnlyBody {
    command: Option<String>,
    error: Option<String>,
}

// ─── Router ─────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<CoreState>> {
    Router::new()
        .route("/system/config", get(get_config).put(set_config))
        .route("/system/reload-polling", post(reload_polling))
        .route("/system/reset", post(reset_all))
        .route("/system/data-dir", get(get_data_dir))
        .route("/system/visible-subscriptions", axum::routing::put(set_visible_subscriptions))
        .route("/system/theme-bg/:theme_id", get(get_theme_bg).delete(remove_theme_bg))
        .route("/system/read-file", get(read_file_base64))
        .route("/system/desktop-only", post(desktop_only_noop))
        .route("/icons", get(list_icons))
        .route("/icons/dir", get(get_icons_dir_path))
        .route("/icons/download-logos", post(download_logos))
        .route("/icons/:symbol", post(set_icon).delete(remove_icon))
        .route("/data/export", get(export_data))
        .route("/data/import", post(import_data))
        .route("/dex/pool/:provider/:address", get(lookup_dex_pool))
}

// ─── System Handlers ────────────────────────────────────────────────────────────

/// GET /system/config
async fn get_config(
    State(state): State<Arc<CoreState>>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    let api_port: u16 = state
        .db
        .get_setting("api_port")
        .ok()
        .flatten()
        .and_then(|v: String| v.parse().ok())
        .unwrap_or(8080);

    let unattended_polling = state.polling.is_unattended().await;

    let api_enabled = state
        .db
        .get_setting("api_enabled")
        .ok()
        .flatten()
        .map(|v| v == "1")
        .unwrap_or(false);

    Ok(ApiResponse::ok(SystemConfig {
        api_port,
        unattended_polling,
        api_enabled,
    })
    .into_response())
}

/// PUT /system/config
async fn set_config(
    State(state): State<Arc<CoreState>>,
    Json(body): Json<SetSystemConfig>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    if let Some(port) = body.api_port {
        if port < 1024 {
            return Err(ApiError::bad_request("Port must be between 1024-65535").into_response());
        }
        state
            .db
            .set_setting("api_port", &port.to_string())
            .map_err(|e| ApiError::internal(e).into_response())?;
    }

    if let Some(enabled) = body.unattended_polling {
        state.polling.set_unattended(enabled).await;
    }

    if let Some(enabled) = body.api_enabled {
        state
            .db
            .set_setting("api_enabled", if enabled { "1" } else { "0" })
            .map_err(|e| ApiError::internal(e).into_response())?;
    }

    Ok(ApiResponse::ok(serde_json::json!({ "success": true })).into_response())
}

/// POST /system/reload-polling
async fn reload_polling(
    State(state): State<Arc<CoreState>>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    state.polling.reload();
    Ok(ApiResponse::ok(serde_json::json!({ "success": true })).into_response())
}

/// POST /system/reset
async fn reset_all(
    State(state): State<Arc<CoreState>>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    state
        .db
        .reset_all_data()
        .map_err(|e| ApiError::internal(e).into_response())?;

    state.notification_engine.reload_rules().await;
    state.polling.reload();
    Ok(ApiResponse::ok(serde_json::json!({ "success": true })).into_response())
}

/// GET /system/data-dir
async fn get_data_dir(
    State(state): State<Arc<CoreState>>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    let dir = state.data_dir.to_string_lossy().to_string();
    Ok(ApiResponse::ok(serde_json::json!({ "data_dir": dir })).into_response())
}

// ─── Icon Handlers ──────────────────────────────────────────────────────────────

/// POST /icons/:symbol — accepts raw bytes body, saves as icons/{symbol}.png
async fn set_icon(
    State(state): State<Arc<CoreState>>,
    Path(symbol): Path<String>,
    body: Bytes,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    if body.is_empty() {
        return Err(ApiError::bad_request("Request body is empty").into_response());
    }

    let icon_name = symbol.to_lowercase();
    let icons_dir = state.data_dir.join("icons");

    tokio::fs::create_dir_all(&icons_dir)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create icons directory: {}", e)).into_response())?;

    let dest = icons_dir.join(format!("{}.png", icon_name));
    tokio::fs::write(&dest, &body)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to write icon: {}", e)).into_response())?;

    Ok(ApiResponse::ok(serde_json::json!({ "path": dest.to_string_lossy() })).into_response())
}

/// DELETE /icons/:symbol — removes icons/{symbol}.png
async fn remove_icon(
    State(state): State<Arc<CoreState>>,
    Path(symbol): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    let icon_name = symbol.to_lowercase();
    let dest = state.data_dir.join("icons").join(format!("{}.png", icon_name));

    if dest.exists() {
        tokio::fs::remove_file(&dest)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to delete icon: {}", e)).into_response())?;
    }

    Ok(ApiResponse::ok(serde_json::json!({ "success": true })).into_response())
}

/// GET /icons/dir — return the icons directory absolute path
async fn get_icons_dir_path(
    State(state): State<Arc<CoreState>>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let dir = state.data_dir.join("icons");
    ApiResponse::ok(dir.to_string_lossy().to_string()).into_response()
}

/// GET /icons — list all available icon filenames
async fn list_icons(
    State(state): State<Arc<CoreState>>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    let icons_dir = state.data_dir.join("icons");

    if !icons_dir.exists() {
        return Ok(ApiResponse::ok(Vec::<String>::new()).into_response());
    }

    let mut icons = Vec::new();
    let mut entries = tokio::fs::read_dir(&icons_dir)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to read icons directory: {}", e)).into_response())?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Some(name) = entry.file_name().to_str() {
            icons.push(name.to_string());
        }
    }

    Ok(ApiResponse::ok(icons).into_response())
}

/// POST /icons/download-logos — download logos for all subscriptions
async fn download_logos(
    State(state): State<Arc<CoreState>>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    let icons_dir = state.data_dir.join("icons");

    // Create a broadcast channel for progress reporting
    let (progress_tx, mut progress_rx) =
        tokio::sync::broadcast::channel::<crate::icons::DownloadProgress>(64);

    // Spawn a task that forwards progress events through the event bus
    let event_bus = state.event_bus.clone();
    tokio::spawn(async move {
        while let Ok(progress) = progress_rx.recv().await {
            let _ = event_bus.send(crate::events::AppEvent::LogoDownloadProgress(progress));
        }
    });

    let result = crate::icons::download_all_logos(&state.db, &icons_dir, Some(progress_tx))
        .await
        .map_err(|e| ApiError::internal(e).into_response())?;

    Ok(ApiResponse::ok(result).into_response())
}

// ─── Data Handlers ──────────────────────────────────────────────────────────────

/// PUT /system/visible-subscriptions
/// Set visible subscription IDs for polling priority.
async fn set_visible_subscriptions(
    State(state): State<Arc<CoreState>>,
    Json(body): Json<SetVisibleSubscriptionsRequest>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    let window_id = body.scope.unwrap_or_else(|| "web".to_string());
    let id_set: std::collections::HashSet<i64> = body.ids.into_iter().collect();
    state.polling.set_visible(window_id, id_set).await;
    Ok(ApiResponse::ok(serde_json::json!({ "success": true })).into_response())
}

/// GET /system/theme-bg/:theme_id
/// Get the theme background file as a base64 data URL (or null if not set).
async fn get_theme_bg(
    State(state): State<Arc<CoreState>>,
    Path(theme_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    let dir = state.data_dir.join("theme_bg");
    for ext in &["png", "jpg", "jpeg", "webp", "img"] {
        let path = dir.join(format!("{}.{}", theme_id, ext));
        if path.exists() {
            match tokio::fs::read(&path).await {
                Ok(bytes) => {
                    use base64::Engine;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    let mime = match *ext {
                        "png" => "image/png",
                        "jpg" | "jpeg" => "image/jpeg",
                        "webp" => "image/webp",
                        _ => "application/octet-stream",
                    };
                    let data_url = format!("data:{};base64,{}", mime, b64);
                    return Ok(ApiResponse::ok(serde_json::json!({ "path": path.to_string_lossy(), "data_url": data_url })).into_response());
                }
                Err(e) => return Err(ApiError::internal(format!("Failed to read theme bg: {}", e)).into_response()),
            }
        }
    }
    Ok(ApiResponse::ok(serde_json::Value::Null).into_response())
}

/// DELETE /system/theme-bg/:theme_id
/// Remove the theme background file.
async fn remove_theme_bg(
    State(state): State<Arc<CoreState>>,
    Path(theme_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    let dir = state.data_dir.join("theme_bg");
    for ext in &["png", "jpg", "jpeg", "webp", "img"] {
        let path = dir.join(format!("{}.{}", theme_id, ext));
        let _ = tokio::fs::remove_file(&path).await;
    }
    Ok(ApiResponse::ok(serde_json::json!({ "success": true })).into_response())
}

/// GET /system/read-file?path=...
/// Read a local file and return its content as a base64 data URL.
async fn read_file_base64(
    Query(query): Query<ReadFileQuery>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    let path = &query.path;
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| ApiError::not_found(format!("Failed to read file: {}", e)).into_response())?;

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let mime = if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".webp") {
        "image/webp"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".gif") {
        "image/gif"
    } else {
        "application/octet-stream"
    };
    let data_url = format!("data:{};base64,{}", mime, b64);
    Ok(ApiResponse::ok(data_url).into_response())
}

/// POST /system/desktop-only
/// Returns a structured error for commands that require desktop (file dialogs, etc.).
async fn desktop_only_noop(
    Json(body): Json<DesktopOnlyBody>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let command = body.command.unwrap_or_else(|| "unknown".to_string());
    let message = body.error.unwrap_or_else(|| {
        format!("'{}' requires desktop mode (native file dialog)", command)
    });
    ApiError::bad_request(message).into_response()
}

// ─── Data Export/Import Handlers ────────────────────────────────────────────────

/// GET /data/export
async fn export_data(
    State(state): State<Arc<CoreState>>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    match state.db.export_data() {
        Ok(data) => Ok(ApiResponse::ok(data).into_response()),
        Err(e) => Err(ApiError::internal(e).into_response()),
    }
}

/// POST /data/import
async fn import_data(
    State(state): State<Arc<CoreState>>,
    Json(data): Json<ExportData>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    match state.db.import_data(&data) {
        Ok((imported, views_imported)) => {
            state.polling.reload();
            Ok(ApiResponse::ok(serde_json::json!({
                "imported_subscriptions": imported,
                "imported_views": views_imported,
            }))
            .into_response())
        }
        Err(e) => Err(ApiError::internal(e).into_response()),
    }
}

// ─── DEX Handler ────────────────────────────────────────────────────────────────

/// GET /dex/pool/:provider/:address — lookup a DEX pool
async fn lookup_dex_pool(
    State(state): State<Arc<CoreState>>,
    Path((provider_id, address)): Path<(String, String)>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    // Get provider settings for API key/URL
    let settings = state.db.get_provider_settings(&provider_id).ok().flatten();
    let api_key = settings.as_ref().and_then(|s| s.api_key.clone());
    let api_url = settings.as_ref().and_then(|s| s.api_url.clone());

    let lookup = create_dex_lookup(&provider_id, api_key, api_url)
        .ok_or_else(|| {
            ApiError::bad_request(format!("Provider '{}' does not support pool lookup", provider_id))
                .into_response()
        })?;

    match lookup.lookup_pool(&address).await {
        Ok(info) => Ok(ApiResponse::ok(info).into_response()),
        Err(e) => Err(ApiError::internal(e).into_response()),
    }
}
