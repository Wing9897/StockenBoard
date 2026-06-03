use crate::db::{DbPool, ExportData, ProviderSettingsRow, Subscription, ViewRow, ViewSubCount};
use crate::events::AppEvent;
use crate::notifications::global_cooldown::GlobalCooldown;
use crate::polling::{PollTick, PollingManager};
use crate::providers::{
    create_dex_lookup, create_ws_provider, get_all_provider_info, registry::ProviderRegistry,
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
    /// 推播通知引擎（規則 CRUD 後需 reload）
    pub notification_engine: Arc<crate::notifications::engine::NotificationEngine>,
    /// AI 排程器（管理 AI 規則的定期評估 task）
    pub ai_scheduler: Arc<crate::notifications::ai_scheduler::AiScheduler>,
    /// 全局通知冷卻期（跨規則共享的最小觸發間隔）
    pub global_cooldown: Arc<GlobalCooldown>,
    ws_sender: broadcast::Sender<WsTickerUpdate>,
    #[allow(clippy::type_complexity)]
    ws_tasks: RwLock<HashMap<String, (tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>)>>,
    pub polling: PollingManager,
}

impl AppState {
    pub fn new(
        db: Arc<DbPool>,
        registry: Arc<ProviderRegistry>,
        event_bus: broadcast::Sender<AppEvent>,
        notification_engine: Arc<crate::notifications::engine::NotificationEngine>,
        ai_scheduler: Arc<crate::notifications::ai_scheduler::AiScheduler>,
        global_cooldown: Arc<GlobalCooldown>,
    ) -> Self {
        let (ws_sender, _) = broadcast::channel(256);
        Self {
            db,
            registry,
            event_bus,
            notification_engine,
            ai_scheduler,
            global_cooldown,
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
            notification_engine: self.notification_engine.clone(),
            ai_scheduler: self.ai_scheduler.clone(),
            global_cooldown: self.global_cooldown.clone(),
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
pub async fn list_all_subscriptions(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Subscription>, String> {
    state.db.list_all_subscriptions()
}

#[tauri::command]
// 引數對應前端 IPC 契約與 subscriptions 資料表欄位，刻意保持平面簽章（見 Requirement 6.4）
#[allow(clippy::too_many_arguments)]
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
    use crate::providers::normalize_symbol;
    let normalized = if sub_type == "dex" {
        symbol.clone()
    } else {
        normalize_symbol(&symbol, &asset_type)
    };
    let id = state.db.add_subscription(
        &sub_type,
        &normalized,
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
        use crate::providers::normalize_symbol;
        let normalized = normalize_symbol(&item.symbol, &item.asset_type);
        match state.db.add_subscription(
            "asset",
            &normalized,
            item.display_name.as_deref(),
            &item.provider_id,
            &item.asset_type,
            None,
            None,
            None,
        ) {
            Ok(_) => succeeded.push(normalized),
            Err(e) if e.contains("已存在") => duplicates.push(normalized),
            Err(_) => failed.push(normalized),
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
    use crate::providers::normalize_symbol;
    let normalized = normalize_symbol(&symbol, &asset_type);
    state.db.update_subscription(
        id,
        &normalized,
        display_name.as_deref(),
        &provider_id,
        &asset_type,
    )?;
    state.polling.reload();
    Ok(())
}

#[tauri::command]
pub async fn remove_subscription(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
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
// 引數對應前端 IPC 契約與 provider_settings 資料表欄位，刻意保持平面簽章
#[allow(clippy::too_many_arguments)]
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

#[tauri::command]
pub async fn reset_all_data(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.db.reset_all_data()?;
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
    let icon_name = symbol.to_lowercase();
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
    let icon_name = symbol.to_lowercase();
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

// ── Logo Batch Download ─────────────────────────────────────────

/// 批量下載訂閱的 logo icon（只下載本地尚未存在的）。
/// 回傳 { succeeded, skipped (已存在), failed (找不到/非 PNG) } 的數量。
#[tauri::command]
pub async fn download_logos(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<LogoDownloadResult, String> {
    use tokio::sync::Semaphore;

    let icons_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?
        .join("icons");
    tokio::fs::create_dir_all(&icons_dir)
        .await
        .map_err(|e| format!("建立 icons 目錄失敗: {}", e))?;

    // 載入所有訂閱
    let subs = state.db.list_all_subscriptions()?;

    let semaphore = std::sync::Arc::new(Semaphore::new(3));
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

        // 已存在 → 跳過（不覆蓋手動設定的）
        if dest.exists() {
            skipped += 1;
            processed += 1;
            let _ = app.emit("logo-download-progress", serde_json::json!({
                "current": processed, "total": total, "symbol": sub.symbol
            }));
            continue;
        }

        let query_symbol = to_query_symbol(&sub.symbol, &sub.asset_type);
        let _permit = semaphore.clone().acquire_owned().await.unwrap();

        // Fallback 鏈
        let bytes = try_download_png(&client, &query_symbol, sub.sub_type == "dex").await;

        drop(_permit);

        match bytes {
            Some(data) => {
                if let Err(e) = tokio::fs::write(&dest, &data).await {
                    eprintln!("[LogoDownload] 寫入 {} 失敗: {}", icon_name, e);
                    failed_list.push(sub.symbol.clone());
                } else {
                    succeeded += 1;
                }
            }
            None => {
                failed_list.push(sub.symbol.clone());
            }
        }

        processed += 1;
        let _ = app.emit("logo-download-progress", serde_json::json!({
            "current": processed, "total": total, "symbol": sub.symbol
        }));

        // 每次請求間隔 200ms，避免觸發 rate limit
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    Ok(LogoDownloadResult {
        succeeded,
        skipped,
        failed: failed_list.len() as u32,
        failed_symbols: failed_list,
    })
}

#[derive(serde::Serialize)]
pub struct LogoDownloadResult {
    pub succeeded: u32,
    pub skipped: u32,
    pub failed: u32,
    pub failed_symbols: Vec<String>,
}

/// 將 symbol 轉為 icon 檔名（直接使用小寫 symbol，不做任何後綴剝離）
fn to_icon_name(symbol: &str) -> String {
    symbol.to_lowercase()
}

/// 將 symbol 轉為用於 logo API 查詢的形式（提取 base symbol）
///
/// 使用 parse_crypto_symbol 通用規則拆分 base/quote，
/// 因為 logo API（Parqet、spothq）只認 base symbol（如 BTC、ETH）。
fn to_query_symbol(symbol: &str, asset_type: &str) -> String {
    match asset_type {
        "crypto" => {
            let (base, _quote) = crate::providers::traits::parse_crypto_symbol(symbol);
            base
        }
        // stock / forex / others: 直接用原始 symbol
        _ => symbol.to_uppercase(),
    }
}

/// 嘗試從多個來源下載 PNG。回傳 Some(bytes) 或 None。
async fn try_download_png(client: &reqwest::Client, symbol: &str, _is_dex: bool) -> Option<Vec<u8>> {
    let upper = symbol.to_uppercase();

    // Parqet (stock + crypto, CDN, 無 rate limit)
    let url = format!("https://assets.parqet.com/logos/symbol/{}", upper);
    fetch_if_png(client, &url).await
}

/// 下載 URL，若 response 是 image/png 或 image/jpeg 則回傳 bytes，否則 None。
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
        return None; // SVG 或其他格式 → 跳過
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.len() < 100 {
        return None; // 太小，可能是空/錯誤頁
    }
    Some(bytes.to_vec())
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
    state.db.get_price_history(
        subscription_id,
        Some(from_ts),
        Some(to_ts),
        limit.unwrap_or(10000),
    )
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

// ── Global Cooldown Commands ────────────────────────────────────

#[tauri::command]
pub async fn get_notification_global_cooldown(
    state: tauri::State<'_, AppState>,
) -> Result<u64, String> {
    let val = state
        .db
        .get_setting("notification_global_cooldown")?
        .unwrap_or_else(|| "30".into());
    val.parse::<u64>()
        .map_err(|e| format!("無效的 cooldown 值: {}", e))
}

#[tauri::command]
pub async fn set_notification_global_cooldown(
    state: tauri::State<'_, AppState>,
    secs: u64,
) -> Result<(), String> {
    state
        .db
        .set_setting("notification_global_cooldown", &secs.to_string())?;
    state.global_cooldown.set_cooldown(secs);
    Ok(())
}

// ── Notification Rule Commands ──────────────────────────────────

#[tauri::command]
pub async fn create_notification_rule(
    state: tauri::State<'_, AppState>,
    rule: crate::notifications::models::CreateRuleRequest,
) -> Result<i64, String> {
    // Validate AI config when condition_type is "ai"
    let threshold = if rule.condition_type == "ai" {
        // ai_config is required for AI rules
        let ai_config = rule
            .ai_config
            .as_ref()
            .ok_or_else(|| "ai_config is required when condition_type is \"ai\"".to_string())?;
        // Validate ai_config fields
        ai_config.validate()?;
        // AI rules use threshold 0.0
        0.0
    } else {
        rule.threshold
    };

    let channel_ids_json = serde_json::to_string(&rule.channel_ids)
        .map_err(|e| format!("序列化 channel_ids 失敗: {}", e))?;
    let cooldown = rule.cooldown_secs.unwrap_or(300) as i64;
    let ai_config_json = rule
        .ai_config
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| format!("序列化 ai_config 失敗: {}", e))?;
    let id = state.db.create_notification_rule(
        &rule.name,
        rule.subscription_id,
        &rule.condition_type,
        threshold,
        &channel_ids_json,
        cooldown,
        ai_config_json.as_deref(),
    )?;
    state.notification_engine.reload_rules().await;
    // Notify AI scheduler to pick up the new rule if it's an AI rule
    if rule.condition_type == "ai" {
        state.ai_scheduler.upsert_rule(id).await;
    }
    Ok(id)
}

#[tauri::command]
pub async fn list_notification_rules(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<crate::db::NotificationRuleRow>, String> {
    state.db.list_notification_rules()
}

#[tauri::command]
pub async fn update_notification_rule(
    state: tauri::State<'_, AppState>,
    id: i64,
    rule: crate::notifications::models::UpdateRuleRequest,
) -> Result<(), String> {
    // Validate AI config if provided
    if let Some(Some(ref ai_config)) = rule.ai_config {
        ai_config.validate()?;
    }

    // If switching to AI type, ensure ai_config is provided
    if let Some(ref ct) = rule.condition_type {
        if ct == "ai" {
            match &rule.ai_config {
                Some(Some(_)) => {} // ai_config provided, OK
                _ => return Err("ai_config is required when condition_type is \"ai\"".to_string()),
            }
        }
    }

    let channel_ids_json = rule
        .channel_ids
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| format!("序列化 channel_ids 失敗: {}", e))?;

    // ai_config: Option<Option<AiConfig>> -> Option<Option<String>>
    // Some(Some(cfg)) => set ai_config to JSON string
    // Some(None) => set ai_config to NULL
    // None => don't update ai_config
    let ai_config_json: Option<Option<String>> = match &rule.ai_config {
        Some(Some(cfg)) => {
            let json =
                serde_json::to_string(cfg).map_err(|e| format!("序列化 ai_config 失敗: {}", e))?;
            Some(Some(json))
        }
        Some(None) => Some(None),
        None => None,
    };

    // If switching to AI type, set threshold to 0.0
    let threshold = if rule.condition_type.as_deref() == Some("ai") {
        Some(0.0)
    } else {
        rule.threshold
    };

    state.db.update_notification_rule(
        id,
        rule.name.as_deref(),
        rule.condition_type.as_deref(),
        threshold,
        channel_ids_json.as_deref(),
        rule.cooldown_secs.map(|s| s as i64),
        ai_config_json.as_ref().map(|opt| opt.as_deref()),
    )?;
    state.notification_engine.reload_rules().await;

    // Notify AI scheduler about the update
    // If switching to AI or updating AI config, upsert the rule
    // If switching away from AI (ai_config set to None), remove the rule
    match &rule.ai_config {
        Some(None) => {
            // Clearing ai_config - remove from scheduler
            state.ai_scheduler.remove_rule(id).await;
        }
        Some(Some(_)) => {
            // AI config updated or switching to AI - upsert
            state.ai_scheduler.upsert_rule(id).await;
        }
        None => {
            // ai_config not being updated, but condition_type might have changed
            if let Some(ref ct) = rule.condition_type {
                if ct != "ai" {
                    // Switching away from AI type
                    state.ai_scheduler.remove_rule(id).await;
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_notification_rule(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    state.db.delete_notification_rule(id)?;
    state.notification_engine.reload_rules().await;
    // Notify AI scheduler to stop any running task for this rule
    state.ai_scheduler.remove_rule(id).await;
    Ok(())
}

#[tauri::command]
pub async fn toggle_notification_rule(
    state: tauri::State<'_, AppState>,
    id: i64,
    enabled: bool,
) -> Result<(), String> {
    state.db.toggle_notification_rule(id, enabled)?;
    state.notification_engine.reload_rules().await;
    // Notify AI scheduler about the toggle
    if enabled {
        // Re-enable: upsert will start the task if it's an AI rule
        state.ai_scheduler.upsert_rule(id).await;
    } else {
        // Disable: remove the task from scheduler
        state.ai_scheduler.remove_rule(id).await;
    }
    Ok(())
}

// ── Notification Channel Commands ───────────────────────────────

#[tauri::command]
pub async fn save_notification_channel(
    state: tauri::State<'_, AppState>,
    channel: crate::notifications::models::SaveChannelRequest,
) -> Result<i64, String> {
    // Validate config based on channel_type
    match channel.channel_type.as_str() {
        "telegram" => {
            let config: crate::notifications::models::TelegramConfig =
                serde_json::from_str(&channel.config)
                    .map_err(|e| format!("Telegram 設定格式無效: {}", e))?;
            if config.bot_token.is_empty() || config.chat_id.is_empty() {
                return Err("Bot Token 和 Chat ID 不可為空".to_string());
            }
            // Encrypt bot_token before storing
            let encrypted_token = crate::notifications::crypto::encrypt_token(&config.bot_token)?;
            let stored_config = serde_json::json!({
                "bot_token": encrypted_token,
                "chat_id": config.chat_id,
            });
            state.db.create_notification_channel(
                &channel.channel_type,
                &channel.name,
                &stored_config.to_string(),
            )
        }
        "webhook" => {
            let config: crate::notifications::models::WebhookConfig =
                serde_json::from_str(&channel.config)
                    .map_err(|e| format!("Webhook 設定格式無效: {}", e))?;
            if config.url.is_empty() {
                return Err("Webhook URL 不可為空".to_string());
            }
            state.db.create_notification_channel(
                &channel.channel_type,
                &channel.name,
                &channel.config,
            )
        }
        _ => Err(format!("不支援的通道類型: {}", channel.channel_type)),
    }
}

#[tauri::command]
pub async fn list_notification_channels(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<crate::db::NotificationChannelRow>, String> {
    state.db.list_notification_channels()
}

#[tauri::command]
pub async fn delete_notification_channel(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    state.db.delete_notification_channel(id)
}

#[tauri::command]
pub async fn test_notification_channel(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let channels = state.db.list_notification_channels()?;
    let channel = channels
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("通道 {} 不存在", id))?;

    let client = reqwest::Client::new();

    match channel.channel_type.as_str() {
        "telegram" => {
            let stored_config: serde_json::Value = serde_json::from_str(&channel.config)
                .map_err(|e| format!("設定解析失敗: {}", e))?;
            let encrypted_token = stored_config["bot_token"]
                .as_str()
                .ok_or("缺少 bot_token")?;
            let chat_id = stored_config["chat_id"].as_str().ok_or("缺少 chat_id")?;
            let bot_token = crate::notifications::crypto::decrypt_token(encrypted_token)?;
            let config = crate::notifications::models::TelegramConfig {
                bot_token,
                chat_id: chat_id.to_string(),
            };
            let test_message =
                "🔔 StockenBoard 測試通知\n\n這是一則測試訊息，確認 Telegram 通道設定正確。";
            crate::notifications::telegram::send_telegram(&client, &config, test_message).await
        }
        "webhook" => {
            let config: crate::notifications::models::WebhookConfig =
                serde_json::from_str(&channel.config)
                    .map_err(|e| format!("設定解析失敗: {}", e))?;
            let test_data = crate::notifications::models::NotificationData {
                symbol: "TEST/USD".to_string(),
                provider: "test".to_string(),
                price: 100.0,
                condition_type: crate::notifications::models::ConditionType::PriceAbove,
                threshold: 99.0,
                rule_name: "測試規則".to_string(),
                triggered_at: chrono::Utc::now(),
            };
            crate::notifications::webhook::send_webhook(&client, &config, &test_data).await
        }
        _ => Err(format!("不支援的通道類型: {}", channel.channel_type)),
    }
}

// ── Notification History Commands ───────────────────────────────

#[tauri::command]
pub async fn get_notification_history(
    state: tauri::State<'_, AppState>,
    rule_id: Option<i64>,
    from: Option<i64>,
    to: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<crate::db::NotificationHistoryRow>, String> {
    state
        .db
        .query_notification_history(rule_id, from, to, limit)
}

// ── AI Provider Config Commands ─────────────────────────────────

#[tauri::command]
pub async fn save_ai_provider_config(
    state: tauri::State<'_, AppState>,
    base_url: String,
    model: String,
    api_key: Option<String>,
) -> Result<(), String> {
    state
        .db
        .save_ai_provider_config(&base_url, &model, api_key.as_deref())?;
    state.ai_scheduler.reload().await;
    Ok(())
}

#[tauri::command]
pub async fn get_ai_provider_config(
    state: tauri::State<'_, AppState>,
) -> Result<Option<crate::notifications::models::AiProviderConfigResponse>, String> {
    let config = state.db.load_ai_provider_config()?;
    Ok(
        config.map(|c| crate::notifications::models::AiProviderConfigResponse {
            base_url: c.base_url,
            model: c.model,
            has_api_key: c.api_key.is_some(),
        }),
    )
}

#[tauri::command]
pub async fn test_ai_connection(state: tauri::State<'_, AppState>) -> Result<String, String> {
    // 1. Load AI provider config from DB
    let config = state
        .db
        .load_ai_provider_config()?
        .ok_or_else(|| "AI provider 尚未設定，請先設定 base_url 和 model".to_string())?;

    // 2. Build the test request URL
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    // 3. Build the request body
    let body = serde_json::json!({
        "model": config.model,
        "messages": [{"role": "user", "content": "Hello"}],
        "max_tokens": 5
    });

    // 4. Create HTTP client with 10s timeout and send request
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("建立 HTTP client 失敗: {}", e))?;

    let mut request = client.post(&url).json(&body);

    // Include Authorization header if api_key is set
    if let Some(ref api_key) = config.api_key {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("連線失敗: {}", e))?;

    // 5. Check response status
    let status = response.status();
    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return Err(format!(
            "AI API 回傳錯誤 (HTTP {}): {}",
            status.as_u16(),
            error_body
        ));
    }

    // 6. Parse response to extract model name
    let resp_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析回應失敗: {}", e))?;

    let model_name = resp_json
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or(&config.model);

    Ok(format!("連線成功！模型: {}", model_name))
}

#[tauri::command]
pub async fn list_ai_models(base_url: String, api_key: Option<String>) -> Result<Vec<String>, String> {
    // Try Ollama-style /api/tags endpoint first, then OpenAI-style /models
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("建立 HTTP client 失敗: {}", e))?;

    let trimmed_url = base_url.trim_end_matches('/');

    // Try Ollama native API: {base_url without /v1}/api/tags
    let ollama_base = trimmed_url.trim_end_matches("/v1");
    let ollama_url = format!("{}/api/tags", ollama_base);

    if let Ok(resp) = client.get(&ollama_url).send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(models) = json.get("models").and_then(|m| m.as_array()) {
                    let names: Vec<String> = models
                        .iter()
                        .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                        .collect();
                    if !names.is_empty() {
                        return Ok(names);
                    }
                }
            }
        }
    }

    // Try OpenAI-compatible /models endpoint
    let openai_url = format!("{}/models", trimmed_url);
    let mut req = client.get(&openai_url);
    if let Some(ref key) = api_key {
        if !key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
    }

    if let Ok(resp) = req.send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                    let names: Vec<String> = data
                        .iter()
                        .filter_map(|m| m.get("id").and_then(|n| n.as_str()).map(|s| s.to_string()))
                        .collect();
                    if !names.is_empty() {
                        return Ok(names);
                    }
                }
            }
        }
    }

    Err("無法取得模型列表，請確認 URL 是否正確".to_string())
}
