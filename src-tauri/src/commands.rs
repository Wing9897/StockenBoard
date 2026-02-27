use crate::polling::{PollTick, PollingManager};
use crate::providers::{
    create_dex_lookup, create_provider_with_url, create_ws_provider,
    get_all_provider_info, AssetData, DataProvider, DexPoolInfo, ProviderInfo, WsTickerUpdate,
};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::{broadcast, RwLock};

pub struct AppState {
    /// On-demand provider instances（用於前端驗證 symbol 等即時查詢）
    providers: RwLock<HashMap<String, Arc<dyn DataProvider>>>,
    ws_sender: broadcast::Sender<WsTickerUpdate>,
    ws_tasks: RwLock<HashMap<String, (tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>)>>,
    pub polling: PollingManager,
    db_path: std::sync::RwLock<Option<std::path::PathBuf>>,
}

impl AppState {
    pub fn new() -> Self {
        let (ws_sender, _) = broadcast::channel(256);
        Self {
            providers: RwLock::new(HashMap::new()),
            ws_sender,
            ws_tasks: RwLock::new(HashMap::new()),
            polling: PollingManager::new(),
            db_path: std::sync::RwLock::new(None),
        }
    }

    pub fn set_db_path(&self, path: std::path::PathBuf) {
        *self.db_path.write().unwrap() = Some(path);
    }

    /// 從 DB 讀取 provider 的 api_key / api_secret / api_url
    fn read_provider_settings(db_path: &std::path::Path, provider_id: &str) -> (Option<String>, Option<String>, Option<String>) {
        let conn = match rusqlite::Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
            Ok(c) => c,
            Err(_) => return (None, None, None),
        };
        let mut stmt = match conn.prepare("SELECT api_key, api_secret, api_url FROM provider_settings WHERE provider_id = ?1") {
            Ok(s) => s,
            Err(_) => return (None, None, None),
        };
        match stmt.query_row([provider_id], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        }) {
            Ok((key, secret, url)) => (
                key.filter(|k| !k.is_empty()),
                secret.filter(|s| !s.is_empty()),
                url.filter(|u| !u.is_empty()),
            ),
            Err(_) => (None, None, None),
        }
    }

    /// 取得或建立 provider instance（lazy，自動從 DB 讀取 API key）
    async fn get_provider(
        &self,
        id: &str,
        api_key: Option<String>,
        api_secret: Option<String>,
    ) -> Option<Arc<dyn DataProvider>> {
        {
            let p = self.providers.read().await;
            if let Some(provider) = p.get(id) {
                return Some(provider.clone());
            }
        }
        // 如果呼叫者沒提供 key，嘗試從 DB 讀取
        let (key, secret, url) = if api_key.is_none() {
            if let Some(ref db_path) = *self.db_path.read().unwrap() {
                Self::read_provider_settings(db_path, id)
            } else {
                (None, None, None)
            }
        } else {
            (api_key, api_secret, None)
        };
        let provider = crate::providers::create_provider_with_url(id, key, secret, url)?;
        self.providers.write().await.insert(id.to_string(), provider.clone());
        Some(provider)
    }
}

// ── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub async fn fetch_asset_price(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    symbol: String,
) -> Result<AssetData, String> {
    let p = state
        .get_provider(&provider_id, None, None)
        .await
        .ok_or_else(|| format!("找不到數據源: {}", provider_id))?;
    p.fetch_price(&symbol).await
}

#[tauri::command]
pub async fn fetch_multiple_prices(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    symbols: Vec<String>,
) -> Result<Vec<AssetData>, String> {
    let p = state
        .get_provider(&provider_id, None, None)
        .await
        .ok_or_else(|| format!("找不到數據源: {}", provider_id))?;
    p.fetch_prices(&symbols).await
}

#[tauri::command]
pub fn get_all_providers() -> Vec<ProviderInfo> {
    get_all_provider_info()
}

#[tauri::command]
pub async fn enable_provider(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    api_key: Option<String>,
    api_secret: Option<String>,
) -> Result<(), String> {
    // 也從 DB 讀取 api_url，確保 DEX provider 能用自訂端點
    let api_url = if let Some(ref db_path) = *state.db_path.read().unwrap() {
        let (_, _, url) = AppState::read_provider_settings(db_path, &provider_id);
        url
    } else {
        None
    };
    if let Some(p) = create_provider_with_url(&provider_id, api_key, api_secret, api_url) {
        state.providers.write().await.insert(provider_id, p);
    }
    state.polling.reload();
    Ok(())
}

#[tauri::command]
pub async fn reload_polling(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.polling.reload();
    Ok(())
}

#[tauri::command]
pub async fn set_unattended_polling(
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state.polling.set_unattended(enabled).await;
    Ok(())
}

#[tauri::command]
pub async fn get_unattended_polling(
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    Ok(state.polling.is_unattended().await)
}

#[tauri::command]
pub async fn set_visible_subscriptions(
    state: tauri::State<'_, AppState>,
    window: tauri::Window,
    ids: Vec<i64>,
    scope: Option<String>,
) -> Result<(), String> {
    let window_id = match scope {
        Some(s) => format!("{}_{}", window.label(), s),
        None => window.label().to_string(),
    };
    let id_set: std::collections::HashSet<i64> = ids.into_iter().collect();
    state.polling.set_visible(window_id, id_set).await;
    Ok(())
}

#[tauri::command]
pub async fn lookup_dex_pool(
    app: tauri::AppHandle,
    provider_id: String,
    pool_address: String,
) -> Result<DexPoolInfo, String> {
    let db_path = app.path().app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("stockenboard.db");
    let (api_key, _, api_url) = AppState::read_provider_settings(&db_path, &provider_id);
    let lookup = create_dex_lookup(&provider_id, api_key, api_url)
        .ok_or_else(|| format!("{} 不支援 pool 查詢", provider_id))?;
    lookup.lookup_pool(&pool_address).await
}

#[tauri::command]
pub async fn get_cached_prices(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AssetData>, String> {
    Ok(state.polling.cache.read().await.values().cloned().collect())
}

#[tauri::command]
pub async fn get_poll_ticks(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PollTick>, String> {
    Ok(state.polling.ticks.read().await.values().cloned().collect())
}

// ── WebSocket ───────────────────────────────────────────────────

#[tauri::command]
pub async fn start_ws_stream(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    provider_id: String,
    symbols: Vec<String>,
) -> Result<(), String> {
    {
        let mut tasks = state.ws_tasks.write().await;
        if let Some((fwd, ws)) = tasks.remove(&provider_id) {
            fwd.abort();
            ws.abort();
        }
    }
    let ws_provider =
        create_ws_provider(&provider_id).ok_or_else(|| format!("{} 不支援 WebSocket", provider_id))?;
    let sender = Arc::new(state.ws_sender.clone());
    let mut receiver = state.ws_sender.subscribe();
    let ws_handle = ws_provider.subscribe(symbols, sender).await?;
    let app_handle = app.clone();
    let forwarder = tokio::spawn(async move {
        while let Ok(update) = receiver.recv().await {
            let _ = app_handle.emit("ws-ticker-update", &update);
        }
    });
    state.ws_tasks.write().await.insert(provider_id, (forwarder, ws_handle));
    Ok(())
}

#[tauri::command]
pub async fn stop_ws_stream(
    state: tauri::State<'_, AppState>,
    provider_id: String,
) -> Result<(), String> {
    if let Some((fwd, ws)) = state.ws_tasks.write().await.remove(&provider_id) {
        fwd.abort();
        ws.abort();
    }
    Ok(())
}

// ── Icon Management ─────────────────────────────────────────────

#[tauri::command]
pub async fn set_icon(app: tauri::AppHandle, symbol: String) -> Result<String, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("圖片", &["png", "jpg", "jpeg", "webp", "svg"])
        .set_title("選擇圖示")
        .pick_file()
        .await
        .ok_or_else(|| "已取消".to_string())?;
    let icon_name = symbol.to_lowercase().replace("usdt", "").replace("-usd", "");
    let icons_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("icons");
    tokio::fs::create_dir_all(&icons_dir)
        .await
        .map_err(|e| format!("建立 icons 目錄失敗: {}", e))?;
    let dest = icons_dir.join(format!("{}.png", icon_name));
    tokio::fs::write(&dest, file.read().await)
        .await
        .map_err(|e| format!("寫入圖示失敗: {}", e))?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn remove_icon(app: tauri::AppHandle, symbol: String) -> Result<(), String> {
    let icon_name = symbol.to_lowercase().replace("usdt", "").replace("-usd", "");
    let dest = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("icons")
        .join(format!("{}.png", icon_name));
    if dest.exists() {
        tokio::fs::remove_file(&dest)
            .await
            .map_err(|e| format!("刪除圖示失敗: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_icons_dir(app: tauri::AppHandle) -> Result<String, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("icons");
    Ok(dir.to_string_lossy().to_string())
}

/// 讀取本地檔案並回傳 base64 data URL — 繞過 asset protocol，dev/prod 都能用
#[tauri::command]
pub async fn read_local_file_base64(path: String) -> Result<String, String> {
    let bytes = tokio::fs::read(&path).await
        .map_err(|e| format!("讀取失敗: {}", e))?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    // 根據副檔名推斷 MIME type
    let mime = if path.ends_with(".png") { "image/png" }
        else if path.ends_with(".jpg") || path.ends_with(".jpeg") { "image/jpeg" }
        else if path.ends_with(".webp") { "image/webp" }
        else if path.ends_with(".svg") { "image/svg+xml" }
        else if path.ends_with(".gif") { "image/gif" }
        else { "application/octet-stream" };
    Ok(format!("data:{};base64,{}", mime, b64))
}

// ── Theme Background ────────────────────────────────────────────

#[tauri::command]
pub async fn save_theme_bg(app: tauri::AppHandle, theme_id: String) -> Result<String, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("圖片", &["png", "jpg", "jpeg", "webp"])
        .set_title("選擇背景圖片")
        .pick_file()
        .await
        .ok_or_else(|| "已取消".to_string())?;
    let dir = app.path().app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("theme_bg");
    tokio::fs::create_dir_all(&dir).await
        .map_err(|e| format!("建立目錄失敗: {}", e))?;

    // 取得原始副檔名，保留正確的 MIME type 讓 asset protocol 能正確回傳
    let ext = file.file_name()
        .rsplit('.')
        .next()
        .map(|e| e.to_lowercase())
        .filter(|e| matches!(e.as_str(), "png" | "jpg" | "jpeg" | "webp"))
        .unwrap_or_else(|| "png".to_string());

    // 清除舊檔（可能是不同副檔名）
    for old_ext in &["png", "jpg", "jpeg", "webp", "img"] {
        let old = dir.join(format!("{}.{}", theme_id, old_ext));
        let _ = tokio::fs::remove_file(&old).await;
    }

    let dest = dir.join(format!("{}.{}", theme_id, ext));
    tokio::fs::write(&dest, file.read().await).await
        .map_err(|e| format!("寫入失敗: {}", e))?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn remove_theme_bg(app: tauri::AppHandle, theme_id: String) -> Result<(), String> {
    let dir = app.path().app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("theme_bg");
    // 清除所有可能的副檔名
    for ext in &["png", "jpg", "jpeg", "webp", "img"] {
        let path = dir.join(format!("{}.{}", theme_id, ext));
        let _ = tokio::fs::remove_file(&path).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_theme_bg_path(app: tauri::AppHandle, theme_id: String) -> Result<Option<String>, String> {
    let dir = app.path().app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("theme_bg");
    // 搜尋所有支援的副檔名
    for ext in &["png", "jpg", "jpeg", "webp", "img"] {
        let path = dir.join(format!("{}.{}", theme_id, ext));
        if path.exists() {
            return Ok(Some(path.to_string_lossy().to_string()));
        }
    }
    Ok(None)
}

// ── Import / Export ─────────────────────────────────────────────

#[tauri::command]
pub async fn export_file(filename: String, content: String) -> Result<(), String> {
    let path = rfd::AsyncFileDialog::new()
        .set_file_name(&filename)
        .add_filter("JSON", &["json"])
        .save_file()
        .await
        .ok_or_else(|| "已取消".to_string())?;
    tokio::fs::write(path.path(), content.as_bytes())
        .await
        .map_err(|e| format!("寫入失敗: {}", e))
}

#[tauri::command]
pub async fn import_file() -> Result<String, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("JSON", &["json"])
        .pick_file()
        .await
        .ok_or_else(|| "已取消".to_string())?;
    String::from_utf8(file.read().await).map_err(|e| format!("讀取失敗: {}", e))
}


// ── Price History ────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct PriceHistoryRecord {
    pub id: i64,
    pub subscription_id: i64,
    pub provider_id: String,
    pub price: f64,
    pub change_pct: Option<f64>,
    pub volume: Option<f64>,
    pub pre_price: Option<f64>,
    pub post_price: Option<f64>,
    pub recorded_at: i64,
}

#[derive(serde::Serialize)]
pub struct HistoryStats {
    pub subscription_id: i64,
    pub total_records: i64,
    pub earliest: Option<i64>,
    pub latest: Option<i64>,
}

#[tauri::command]
pub async fn toggle_record(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    subscription_id: i64,
    enabled: bool,
) -> Result<(), String> {
    let db_path = app.path().app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("stockenboard.db");
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| format!("開啟 DB 失敗: {}", e))?;
        conn.execute(
            "UPDATE subscriptions SET record_enabled = ?1 WHERE id = ?2",
            rusqlite::params![if enabled { 1 } else { 0 }, subscription_id],
        ).map_err(|e| format!("更新失敗: {}", e))?;
        Ok(())
    }).await.map_err(|e| format!("spawn 失敗: {}", e))?;
    // 通知 polling 重新載入，以更新 record_symbols
    state.polling.reload();
    Ok(())
}

#[tauri::command]
pub async fn get_price_history(
    app: tauri::AppHandle,
    subscription_id: i64,
    from_ts: i64,
    to_ts: i64,
    limit: Option<i64>,
) -> Result<Vec<PriceHistoryRecord>, String> {
    let db_path = app.path().app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("stockenboard.db");
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("開啟 DB 失敗: {}", e))?;
        let lim = limit.unwrap_or(10000);
        let mut stmt = conn.prepare(
            "SELECT id, subscription_id, provider_id, price, change_pct, volume, pre_price, post_price, recorded_at \
             FROM price_history WHERE subscription_id = ?1 AND recorded_at >= ?2 AND recorded_at <= ?3 \
             ORDER BY recorded_at ASC LIMIT ?4"
        ).map_err(|e| format!("查詢失敗: {}", e))?;
        let rows = stmt.query_map(rusqlite::params![subscription_id, from_ts, to_ts, lim], |row| {
            Ok(PriceHistoryRecord {
                id: row.get(0)?,
                subscription_id: row.get(1)?,
                provider_id: row.get(2)?,
                price: row.get(3)?,
                change_pct: row.get(4)?,
                volume: row.get(5)?,
                pre_price: row.get(6)?,
                post_price: row.get(7)?,
                recorded_at: row.get(8)?,
            })
        }).map_err(|e| format!("讀取失敗: {}", e))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }).await.map_err(|e| format!("spawn 失敗: {}", e))?
}

#[tauri::command]
pub async fn get_history_stats(
    app: tauri::AppHandle,
    subscription_ids: Vec<i64>,
) -> Result<Vec<HistoryStats>, String> {
    let db_path = app.path().app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("stockenboard.db");
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("開啟 DB 失敗: {}", e))?;
        let mut results = Vec::new();
        for sid in &subscription_ids {
            let mut stmt = conn.prepare_cached(
                "SELECT COUNT(*), MIN(recorded_at), MAX(recorded_at) FROM price_history WHERE subscription_id = ?1"
            ).map_err(|e| format!("查詢失敗: {}", e))?;
            let stat = stmt.query_row([sid], |row| {
                Ok(HistoryStats {
                    subscription_id: *sid,
                    total_records: row.get(0)?,
                    earliest: row.get(1)?,
                    latest: row.get(2)?,
                })
            }).map_err(|e| format!("讀取失敗: {}", e))?;
            results.push(stat);
        }
        Ok(results)
    }).await.map_err(|e| format!("spawn 失敗: {}", e))?
}

#[tauri::command]
pub async fn cleanup_history(
    app: tauri::AppHandle,
    retention_days: Option<i64>,
) -> Result<i64, String> {
    let db_path = app.path().app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("stockenboard.db");
    let days = retention_days.unwrap_or(90);
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| format!("開啟 DB 失敗: {}", e))?;
        let cutoff = chrono::Utc::now().timestamp() - (days * 86400);
        let deleted = conn.execute(
            "DELETE FROM price_history WHERE recorded_at < ?1",
            rusqlite::params![cutoff],
        ).map_err(|e| format!("清理失敗: {}", e))?;
        Ok(deleted as i64)
    }).await.map_err(|e| format!("spawn 失敗: {}", e))?
}

#[tauri::command]
pub async fn purge_all_history(
    app: tauri::AppHandle,
) -> Result<i64, String> {
    let db_path = app.path().app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("stockenboard.db");
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| format!("開啟 DB 失敗: {}", e))?;
        let deleted = conn.execute("DELETE FROM price_history", [])
            .map_err(|e| format!("清除失敗: {}", e))?;
        Ok(deleted as i64)
    }).await.map_err(|e| format!("spawn 失敗: {}", e))?
}

#[tauri::command]
pub async fn get_data_dir(
    app: tauri::AppHandle,
) -> Result<String, String> {
    let dir = app.path().app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?;
    Ok(dir.to_string_lossy().to_string())
}
