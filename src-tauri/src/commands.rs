use crate::db::{DbPool, ExportData, ProviderSettingsRow, Subscription, ViewRow, ViewSubCount};
use crate::events::AppEvent;
use crate::polling::{PollTick, PollingManager};
use crate::providers::{
    create_dex_lookup, create_ws_provider, get_all_provider_info,
    registry::ProviderRegistry,
    AssetData, DexPoolInfo, ProviderInfo, WsTickerUpdate,
};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_shell::ShellExt;
use tokio::sync::{broadcast, RwLock};

// ── AppState ────────────────────────────────────────────────────

pub struct AppState {
    /// 統一 DB 存取層
    pub db: Arc<DbPool>,
    /// 共享 Provider Registry（含 rate limiting）
    pub registry: Arc<ProviderRegistry>,
    /// Event Bus（解耦 Polling ↔ DB ↔ 前端）
    pub event_bus: broadcast::Sender<AppEvent>,
    ws_sender: broadcast::Sender<WsTickerUpdate>,
    #[allow(clippy::type_complexity)]
    ws_tasks: RwLock<HashMap<String, (tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>)>>,
    pub polling: PollingManager,
}

impl AppState {
    pub fn new(db: Arc<DbPool>, registry: Arc<ProviderRegistry>, event_bus: broadcast::Sender<AppEvent>) -> Self {
        let (ws_sender, _) = broadcast::channel(256);
        Self {
            db,
            registry,
            event_bus,
            ws_sender,
            ws_tasks: RwLock::new(HashMap::new()),
            polling: PollingManager::new(),
        }
    }

    /// 創建一個用於 API server 的輕量級 clone
    pub fn clone_for_api(&self) -> Self {
        Self {
            db: self.db.clone(),
            registry: self.registry.clone(),
            event_bus: self.event_bus.clone(),
            ws_sender: broadcast::channel(1).0,
            ws_tasks: RwLock::new(HashMap::new()),
            polling: self.polling.clone(),
        }
    }
}

// ── Provider / Fetch Commands ───────────────────────────────────

#[tauri::command]
pub async fn fetch_asset_price(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    symbol: String,
) -> Result<AssetData, String> {
    let p = state
        .registry
        .get_or_create(&provider_id, &state.db)
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
    state
        .registry
        .fetch_with_limit(&provider_id, &symbols, &state.db)
        .await
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
    let api_url = state
        .db
        .get_provider_settings(&provider_id)
        .ok()
        .flatten()
        .and_then(|s| s.api_url.filter(|u| !u.is_empty()));
    state
        .registry
        .update_provider(&provider_id, api_key, api_secret, api_url)
        .await;
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
pub async fn get_unattended_polling(state: tauri::State<'_, AppState>) -> Result<bool, String> {
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
    state: tauri::State<'_, AppState>,
    provider_id: String,
    pool_address: String,
) -> Result<DexPoolInfo, String> {
    let settings = state.db.get_provider_settings(&provider_id).ok().flatten();
    let api_key = settings.as_ref().and_then(|s| s.api_key.clone());
    let api_url = settings.as_ref().and_then(|s| s.api_url.clone());
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
pub async fn get_poll_ticks(state: tauri::State<'_, AppState>) -> Result<Vec<PollTick>, String> {
    Ok(state.polling.ticks.read().await.values().cloned().collect())
}

// ── Subscription Commands (NEW - 取代前端 SQL) ──────────────────

#[tauri::command]
pub async fn list_subscriptions(
    state: tauri::State<'_, AppState>,
    sub_type: String,
) -> Result<Vec<Subscription>, String> {
    state.db.list_subscriptions(&sub_type)
}

#[tauri::command]
pub async fn add_subscription(
    state: tauri::State<'_, AppState>,
    sub_type: String,
    symbol: String,
    display_name: Option<String>,
    provider_id: String,
    asset_type: String,
    pool_address: Option<String>,
    token_from: Option<String>,
    token_to: Option<String>,
) -> Result<i64, String> {
    let id = state.db.add_subscription(
        &sub_type,
        &symbol,
        display_name.as_deref(),
        &provider_id,
        &asset_type,
        pool_address.as_deref(),
        token_from.as_deref(),
        token_to.as_deref(),
    )?;
    state.polling.reload();
    Ok(id)
}

#[derive(serde::Deserialize)]
pub struct BatchAddItem {
    pub symbol: String,
    pub display_name: Option<String>,
    pub provider_id: String,
    pub asset_type: String,
}

#[derive(serde::Serialize)]
pub struct BatchAddResult {
    pub succeeded: Vec<String>,
    pub failed: Vec<String>,
    pub duplicates: Vec<String>,
}

#[tauri::command]
pub async fn add_subscriptions_batch(
    state: tauri::State<'_, AppState>,
    items: Vec<BatchAddItem>,
) -> Result<BatchAddResult, String> {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    let mut duplicates = Vec::new();

    for item in &items {
        match state.db.add_subscription(
            "asset",
            &item.symbol,
            item.display_name.as_deref(),
            &item.provider_id,
            &item.asset_type,
            None,
            None,
            None,
        ) {
            Ok(_) => succeeded.push(item.symbol.clone()),
            Err(e) if e.contains("已存在") => duplicates.push(item.symbol.clone()),
            Err(_) => failed.push(item.symbol.clone()),
        }
    }

    if !succeeded.is_empty() {
        state.polling.reload();
    }

    Ok(BatchAddResult {
        succeeded,
        failed,
        duplicates,
    })
}

#[tauri::command]
pub async fn update_subscription(
    state: tauri::State<'_, AppState>,
    id: i64,
    symbol: String,
    display_name: Option<String>,
    provider_id: String,
    asset_type: String,
) -> Result<(), String> {
    state
        .db
        .update_subscription(id, &symbol, display_name.as_deref(), &provider_id, &asset_type)?;
    state.polling.reload();
    Ok(())
}

#[tauri::command]
pub async fn remove_subscription(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    state.db.remove_subscription(id)?;
    state.polling.reload();
    Ok(())
}

#[tauri::command]
pub async fn remove_subscriptions(
    state: tauri::State<'_, AppState>,
    ids: Vec<i64>,
) -> Result<(), String> {
    state.db.remove_subscriptions(&ids)?;
    state.polling.reload();
    Ok(())
}

#[tauri::command]
pub async fn has_api_key(
    state: tauri::State<'_, AppState>,
    provider_id: String,
) -> Result<bool, String> {
    Ok(state.db.has_api_key(&provider_id))
}

// ── Provider Settings Commands (NEW) ────────────────────────────

#[tauri::command]
pub async fn list_provider_settings(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ProviderSettingsRow>, String> {
    state.db.list_provider_settings()
}

#[tauri::command]
pub async fn upsert_provider_settings(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    api_key: Option<String>,
    api_secret: Option<String>,
    api_url: Option<String>,
    refresh_interval: Option<i64>,
    connection_type: String,
    record_from_hour: Option<i64>,
    record_to_hour: Option<i64>,
) -> Result<(), String> {
    state.db.upsert_provider_settings(
        &provider_id,
        api_key.as_deref(),
        api_secret.as_deref(),
        api_url.as_deref(),
        refresh_interval,
        &connection_type,
        record_from_hour,
        record_to_hour,
    )?;
    // 同步 Rust 端 provider instance + 觸發 polling reload
    state
        .registry
        .update_provider(
            &provider_id,
            api_key.filter(|k| !k.is_empty()),
            api_secret.filter(|s| !s.is_empty()),
            api_url.filter(|u| !u.is_empty()),
        )
        .await;
    state.polling.reload();
    Ok(())
}

// ── View Commands (NEW) ─────────────────────────────────────────

#[tauri::command]
pub async fn list_views(
    state: tauri::State<'_, AppState>,
    view_type: String,
) -> Result<Vec<ViewRow>, String> {
    state.db.list_views(&view_type)
}

#[tauri::command]
pub async fn create_view(
    state: tauri::State<'_, AppState>,
    name: String,
    view_type: String,
) -> Result<i64, String> {
    state.db.create_view(&name, &view_type)
}

#[tauri::command]
pub async fn rename_view(
    state: tauri::State<'_, AppState>,
    id: i64,
    name: String,
) -> Result<(), String> {
    state.db.rename_view(id, &name)
}

#[tauri::command]
pub async fn delete_view(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    state.db.delete_view(id)
}

#[tauri::command]
pub async fn get_view_sub_counts(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ViewSubCount>, String> {
    state.db.get_view_sub_counts()
}

#[tauri::command]
pub async fn get_view_subscription_ids(
    state: tauri::State<'_, AppState>,
    view_id: i64,
) -> Result<Vec<i64>, String> {
    state.db.get_view_subscription_ids(view_id)
}

#[tauri::command]
pub async fn add_sub_to_view(
    state: tauri::State<'_, AppState>,
    view_id: i64,
    subscription_id: i64,
) -> Result<(), String> {
    state.db.add_sub_to_view(view_id, subscription_id)
}

#[tauri::command]
pub async fn remove_sub_from_view(
    state: tauri::State<'_, AppState>,
    view_id: i64,
    subscription_id: i64,
) -> Result<(), String> {
    state.db.remove_sub_from_view(view_id, subscription_id)
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
    let ws_provider = create_ws_provider(&provider_id)
        .ok_or_else(|| format!("{} 不支援 WebSocket", provider_id))?;
    let sender = Arc::new(state.ws_sender.clone());
    let mut receiver = state.ws_sender.subscribe();
    let ws_handle = ws_provider.subscribe(symbols, sender).await?;
    let app_handle = app.clone();
    let forwarder = tokio::spawn(async move {
        while let Ok(update) = receiver.recv().await {
            let _ = app_handle.emit("ws-ticker-update", &update);
        }
    });
    state
        .ws_tasks
        .write()
        .await
        .insert(provider_id, (forwarder, ws_handle));
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
    let icon_name = symbol
        .to_lowercase()
        .replace("usdt", "")
        .replace("-usd", "");
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
    let icon_name = symbol
        .to_lowercase()
        .replace("usdt", "")
        .replace("-usd", "");
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

/// 讀取本地檔案並回傳 base64 data URL
#[tauri::command]
pub async fn read_local_file_base64(path: String) -> Result<String, String> {
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("讀取失敗: {}", e))?;
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
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("theme_bg");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("建立目錄失敗: {}", e))?;
    let ext = file
        .file_name()
        .rsplit('.')
        .next()
        .map(|e| e.to_lowercase())
        .filter(|e| matches!(e.as_str(), "png" | "jpg" | "jpeg" | "webp"))
        .unwrap_or_else(|| "png".to_string());
    for old_ext in &["png", "jpg", "jpeg", "webp", "img"] {
        let old = dir.join(format!("{}.{}", theme_id, old_ext));
        let _ = tokio::fs::remove_file(&old).await;
    }
    let dest = dir.join(format!("{}.{}", theme_id, ext));
    tokio::fs::write(&dest, file.read().await)
        .await
        .map_err(|e| format!("寫入失敗: {}", e))?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn remove_theme_bg(app: tauri::AppHandle, theme_id: String) -> Result<(), String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("theme_bg");
    for ext in &["png", "jpg", "jpeg", "webp", "img"] {
        let path = dir.join(format!("{}.{}", theme_id, ext));
        let _ = tokio::fs::remove_file(&path).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_theme_bg_path(
    app: tauri::AppHandle,
    theme_id: String,
) -> Result<Option<String>, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("theme_bg");
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

#[tauri::command]
pub async fn export_data(state: tauri::State<'_, AppState>) -> Result<ExportData, String> {
    state.db.export_data()
}

#[tauri::command]
pub async fn import_data(
    state: tauri::State<'_, AppState>,
    data: ExportData,
) -> Result<(usize, usize), String> {
    let result = state.db.import_data(&data)?;
    state.polling.reload();
    Ok(result)
}

// ── Price History ────────────────────────────────────────────────

#[tauri::command]
pub async fn toggle_record(
    state: tauri::State<'_, AppState>,
    subscription_id: i64,
    enabled: bool,
) -> Result<(), String> {
    state.db.toggle_record(subscription_id, enabled)?;
    state.polling.reload();
    Ok(())
}

#[tauri::command]
pub async fn set_record_hours(
    state: tauri::State<'_, AppState>,
    subscription_id: i64,
    from_hour: Option<i64>,
    to_hour: Option<i64>,
) -> Result<(), String> {
    state
        .db
        .set_record_hours(subscription_id, from_hour, to_hour)
}

#[tauri::command]
pub async fn set_provider_record_hours(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    from_hour: Option<i64>,
    to_hour: Option<i64>,
) -> Result<(), String> {
    state
        .db
        .set_provider_record_hours(&provider_id, from_hour, to_hour)
}

#[tauri::command]
pub async fn get_price_history(
    state: tauri::State<'_, AppState>,
    subscription_id: i64,
    from_ts: i64,
    to_ts: i64,
    limit: Option<i64>,
) -> Result<Vec<crate::db::PriceHistoryRow>, String> {
    state
        .db
        .get_price_history(subscription_id, Some(from_ts), Some(to_ts), limit.unwrap_or(10000))
}

#[tauri::command]
pub async fn get_history_stats(
    state: tauri::State<'_, AppState>,
    subscription_ids: Vec<i64>,
) -> Result<Vec<HistoryStatsResult>, String> {
    let mut results = Vec::new();
    for sid in subscription_ids {
        let stats = state.db.get_history_stats(sid)?;
        results.push(HistoryStatsResult {
            subscription_id: sid,
            total_records: stats.total,
            earliest: stats.oldest,
            latest: stats.newest,
        });
    }
    Ok(results)
}

#[derive(serde::Serialize)]
pub struct HistoryStatsResult {
    pub subscription_id: i64,
    pub total_records: i64,
    pub earliest: Option<i64>,
    pub latest: Option<i64>,
}

#[tauri::command]
pub async fn cleanup_history(
    state: tauri::State<'_, AppState>,
    retention_days: Option<i64>,
) -> Result<i64, String> {
    let days = retention_days.unwrap_or(90);
    let cutoff = chrono::Utc::now().timestamp() - (days * 86400);
    state.db.cleanup_history(cutoff)
}

#[tauri::command]
pub async fn purge_all_history(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.db.purge_all_history()
}

#[tauri::command]
pub async fn delete_subscription_history(
    state: tauri::State<'_, AppState>,
    subscription_id: i64,
) -> Result<i64, String> {
    state.db.delete_history_for_subscription(subscription_id)
}

#[tauri::command]
pub async fn get_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        let path_str = dir.to_string_lossy().to_string();
        app.shell()
            .command("explorer")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("無法開啟資料夾: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        let path_str = dir.to_string_lossy().to_string();
        app.shell()
            .command("open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("無法開啟資料夾: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let path_str = dir.to_string_lossy().to_string();
        app.shell()
            .command("xdg-open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("無法開啟資料夾: {}", e))?;
    }

    Ok(dir.to_string_lossy().to_string())
}

// ── API Settings ────────────────────────────────────────────────

#[tauri::command]
pub async fn get_api_port(state: tauri::State<'_, AppState>) -> Result<u16, String> {
    let val = state.db.get_setting("api_port")?.unwrap_or("8080".into());
    val.parse::<u16>()
        .map_err(|e| format!("無效的 port: {}", e))
}

#[tauri::command]
pub async fn set_api_port(state: tauri::State<'_, AppState>, port: u16) -> Result<(), String> {
    if port < 1024 {
        return Err("Port 必須在 1024-65535 之間".to_string());
    }
    state.db.set_setting("api_port", &port.to_string())
}

#[tauri::command]
pub async fn get_api_enabled(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let val = state.db.get_setting("api_enabled")?.unwrap_or("0".into());
    Ok(val == "1")
}

#[tauri::command]
pub async fn set_api_enabled(
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state
        .db
        .set_setting("api_enabled", if enabled { "1" } else { "0" })
}
