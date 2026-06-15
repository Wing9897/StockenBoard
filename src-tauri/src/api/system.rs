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
    extract::{Json, Path, State},
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
}

#[derive(Debug, Deserialize)]
struct SetSystemConfig {
    api_port: Option<u16>,
    unattended_polling: Option<bool>,
}

#[derive(Debug, Serialize)]
struct LogoDownloadResult {
    succeeded: u32,
    skipped: u32,
    failed: u32,
    failed_symbols: Vec<String>,
}

// ─── Router ─────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<CoreState>> {
    Router::new()
        .route("/system/config", get(get_config).put(set_config))
        .route("/system/reload-polling", post(reload_polling))
        .route("/system/reset", post(reset_all))
        .route("/system/data-dir", get(get_data_dir))
        .route("/icons", get(list_icons))
        .route("/icons/download-logos", post(download_logos))
        .route("/icons/{symbol}", post(set_icon).delete(remove_icon))
        .route("/data/export", get(export_data))
        .route("/data/import", post(import_data))
        .route("/dex/pool/{provider}/{address}", get(lookup_dex_pool))
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

    Ok(ApiResponse::ok(SystemConfig {
        api_port,
        unattended_polling,
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
    tokio::fs::create_dir_all(&icons_dir)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create icons directory: {}", e)).into_response())?;

    let subs = state
        .db
        .list_all_subscriptions()
        .map_err(|e| ApiError::internal(e).into_response())?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("StockenBoard/1.0")
        .build()
        .unwrap_or_default();

    let semaphore = Arc::new(tokio::sync::Semaphore::new(3));
    let mut succeeded = 0u32;
    let mut skipped = 0u32;
    let mut failed_list: Vec<String> = Vec::new();

    for sub in &subs {
        let icon_name = sub.symbol.to_lowercase();
        let dest = icons_dir.join(format!("{}.png", icon_name));

        // Already exists → skip
        if dest.exists() {
            skipped += 1;
            continue;
        }

        let query_symbol = to_query_symbol(&sub.symbol, &sub.asset_type);
        let _permit = semaphore.clone().acquire_owned().await.unwrap();

        let bytes = try_download_png(&client, &query_symbol).await;
        drop(_permit);

        match bytes {
            Some(data) => {
                if let Err(_e) = tokio::fs::write(&dest, &data).await {
                    failed_list.push(sub.symbol.clone());
                } else {
                    succeeded += 1;
                }
            }
            None => {
                failed_list.push(sub.symbol.clone());
            }
        }

        // Rate limit protection
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    Ok(ApiResponse::ok(LogoDownloadResult {
        succeeded,
        skipped,
        failed: failed_list.len() as u32,
        failed_symbols: failed_list,
    })
    .into_response())
}

// ─── Data Handlers ──────────────────────────────────────────────────────────────

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

// ─── Helper Functions ───────────────────────────────────────────────────────────

/// Convert symbol to query format for logo API
fn to_query_symbol(symbol: &str, asset_type: &str) -> String {
    match asset_type {
        "crypto" => {
            let (base, _quote) = crate::providers::traits::parse_crypto_symbol(symbol);
            base
        }
        _ => symbol.to_uppercase(),
    }
}

/// Try downloading a PNG from logo sources
async fn try_download_png(client: &reqwest::Client, symbol: &str) -> Option<Vec<u8>> {
    let upper = symbol.to_uppercase();
    let url = format!("https://assets.parqet.com/logos/symbol/{}", upper);
    fetch_if_png(client, &url).await
}

/// Fetch URL, return bytes if response is image/png or image/jpeg
async fn fetch_if_png(client: &reqwest::Client, url: &str) -> Option<Vec<u8>> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !content_type.starts_with("image/png") && !content_type.starts_with("image/jpeg") {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.len() < 100 {
        return None;
    }
    Some(bytes.to_vec())
}
