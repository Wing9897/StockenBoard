//! Shared logo download logic used by both Tauri commands and the HTTP API.

use std::path::Path;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::Semaphore;

use crate::db::DbPool;

/// Result of a bulk logo download operation.
#[derive(Debug, Clone, Serialize)]
pub struct LogoDownloadResult {
    pub succeeded: u32,
    pub skipped: u32,
    pub failed: u32,
    pub failed_symbols: Vec<String>,
}

/// Progress payload emitted during logo downloads.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub current: u32,
    pub total: u32,
    pub symbol: String,
}

/// Download logos for all subscriptions that don't already have a local icon.
///
/// - `db`: database pool to query subscriptions
/// - `icons_dir`: directory where icons are stored (will be created if missing)
/// - `progress_tx`: optional channel to report per-symbol progress
pub async fn download_all_logos(
    db: &DbPool,
    icons_dir: &Path,
    progress_tx: Option<tokio::sync::broadcast::Sender<DownloadProgress>>,
) -> Result<LogoDownloadResult, String> {
    tokio::fs::create_dir_all(icons_dir)
        .await
        .map_err(|e| format!("Failed to create icons directory: {}", e))?;

    let subs = db.list_all_subscriptions()?;

    let semaphore = Arc::new(Semaphore::new(3));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("StockenBoard/1.0")
        .build()
        .unwrap_or_default();

    let mut succeeded = 0u32;
    let mut skipped = 0u32;
    let mut failed_list: Vec<String> = Vec::new();
    let total = subs.len() as u32;
    let mut processed = 0u32;

    for sub in &subs {
        let icon_name = to_icon_name(&sub.symbol);
        let dest = icons_dir.join(format!("{}.png", icon_name));

        // Already exists → skip (don't overwrite manually set icons)
        if dest.exists() {
            skipped += 1;
            processed += 1;
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(DownloadProgress {
                    current: processed,
                    total,
                    symbol: sub.symbol.clone(),
                });
            }
            continue;
        }

        let query_symbol = to_query_symbol(&sub.symbol, &sub.asset_type);
        let _permit = semaphore.clone().acquire_owned().await.unwrap();

        let bytes = try_download_png(&client, &query_symbol, sub.sub_type == "dex").await;

        match bytes {
            Some(data) => {
                if let Err(e) = tokio::fs::write(&dest, &data).await {
                    eprintln!("[LogoDownload] Failed to write {}: {}", icon_name, e);
                    failed_list.push(sub.symbol.clone());
                } else {
                    succeeded += 1;
                }
            }
            None => {
                failed_list.push(sub.symbol.clone());
            }
        }

        // Hold the permit until after file write completes to enforce concurrency limit during I/O
        drop(_permit);

        processed += 1;
        if let Some(ref tx) = progress_tx {
            let _ = tx.send(DownloadProgress {
                current: processed,
                total,
                symbol: sub.symbol.clone(),
            });
        }

        // Rate limit protection: 200ms between requests
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    Ok(LogoDownloadResult {
        succeeded,
        skipped,
        failed: failed_list.len() as u32,
        failed_symbols: failed_list,
    })
}

/// Convert a symbol to its canonical icon filename (without extension).
pub fn to_icon_name(symbol: &str) -> String {
    symbol.to_lowercase()
}

/// Convert a symbol to the query format expected by logo APIs.
pub fn to_query_symbol(symbol: &str, asset_type: &str) -> String {
    match asset_type {
        "crypto" => {
            let (base, _quote) = crate::providers::traits::parse_crypto_symbol(symbol);
            base
        }
        // stock / forex / others: use raw symbol uppercased
        _ => symbol.to_uppercase(),
    }
}

/// Try downloading a PNG logo from known sources.
pub async fn try_download_png(
    client: &reqwest::Client,
    symbol: &str,
    _is_dex: bool,
) -> Option<Vec<u8>> {
    let upper = symbol.to_uppercase();

    // Parqet (stock + crypto, CDN, no rate limit)
    let url = format!("https://assets.parqet.com/logos/symbol/{}", upper);
    fetch_if_png(client, &url).await
}

/// Fetch URL, return bytes only if response is image/png or image/jpeg and large enough.
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
        return None; // SVG or other format → skip
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.len() < 100 {
        return None; // Too small, likely empty or error page
    }
    Some(bytes.to_vec())
}
