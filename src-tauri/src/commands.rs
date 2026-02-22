use crate::providers::{
    AssetData, DataProvider, ProviderInfo, WsTickerUpdate,
    get_all_provider_info, create_provider, create_ws_provider,
};
use tauri::Emitter;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

pub struct AppState {
    pub providers: RwLock<HashMap<String, Arc<dyn DataProvider>>>,
    pub ws_sender: broadcast::Sender<WsTickerUpdate>,
    /// 追蹤每個 provider 的 WS 相關 tasks：(forwarder_task, ws_connection_task)
    pub ws_tasks: RwLock<HashMap<String, (tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>)>>,
}

impl AppState {
    pub fn new() -> Self {
        let (ws_sender, _) = broadcast::channel(256);

        // 啟動時先建立免費 provider（不需要 key 的）
        let mut providers: HashMap<String, Arc<dyn DataProvider>> = HashMap::new();
        for id in ["binance", "coinbase", "coingecko", "yahoo", "cryptocompare", "polymarket"] {
            if let Some(p) = create_provider(id, None, None) {
                providers.insert(id.to_string(), p);
            }
        }

        Self {
            providers: RwLock::new(providers),
            ws_sender,
            ws_tasks: RwLock::new(HashMap::new()),
        }
    }

    /// 從 DB 讀取已儲存的 API key，重新初始化對應的 provider
    pub fn init_from_db_sync(&self, app_dir: &std::path::Path) {
        let db_path = app_dir.join("stockenboard.db");
        if !db_path.exists() { return; }

        let conn = match rusqlite::Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut stmt = match conn.prepare(
            "SELECT provider_id, api_key, api_secret FROM provider_settings WHERE api_key IS NOT NULL AND api_key != ''"
        ) {
            Ok(s) => s,
            Err(_) => return,
        };

        let rows: Vec<(String, Option<String>, Option<String>)> = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        }).ok()
            .map(|r| r.flatten().collect())
            .unwrap_or_default();

        if let Ok(mut providers) = self.providers.try_write() {
            for (id, api_key, api_secret) in rows {
                if let Some(p) = create_provider(&id, api_key, api_secret) {
                    providers.insert(id, p);
                }
            }
        }
    }

    pub async fn get_or_create_provider(
        &self, provider_id: &str, api_key: Option<String>, api_secret: Option<String>,
    ) -> Option<Arc<dyn DataProvider>> {
        {
            let providers = self.providers.read().await;
            if let Some(p) = providers.get(provider_id) {
                return Some(p.clone());
            }
        }
        if let Some(provider) = create_provider(provider_id, api_key, api_secret) {
            let mut providers = self.providers.write().await;
            providers.insert(provider_id.to_string(), provider.clone());
            return Some(provider);
        }
        None
    }
}

#[tauri::command]
pub async fn fetch_asset_price(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    symbol: String,
) -> Result<AssetData, String> {
    let provider = state.get_or_create_provider(&provider_id, None, None).await
        .ok_or_else(|| format!("找不到數據源: {}", provider_id))?;
    provider.fetch_price(&symbol).await
}

#[tauri::command]
pub async fn fetch_multiple_prices(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    symbols: Vec<String>,
) -> Result<Vec<AssetData>, String> {
    let provider = state.get_or_create_provider(&provider_id, None, None).await
        .ok_or_else(|| format!("找不到數據源: {}", provider_id))?;
    provider.fetch_prices(&symbols).await
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
    if let Some(provider) = create_provider(&provider_id, api_key, api_secret) {
        let mut providers = state.providers.write().await;
        providers.insert(provider_id, provider);
    }
    Ok(())
}

#[tauri::command]
pub async fn disable_provider(
    state: tauri::State<'_, AppState>,
    provider_id: String,
) -> Result<(), String> {
    let mut providers = state.providers.write().await;
    providers.remove(&provider_id);
    Ok(())
}

/// Start a WebSocket connection for real-time data
#[tauri::command]
pub async fn start_ws_stream(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    provider_id: String,
    symbols: Vec<String>,
) -> Result<(), String> {
    // Stop existing WS for this provider
    {
        let mut tasks = state.ws_tasks.write().await;
        if let Some((forwarder, ws_conn)) = tasks.remove(&provider_id) {
            forwarder.abort();
            ws_conn.abort();
        }
    }

    let ws_provider = create_ws_provider(&provider_id)
        .ok_or_else(|| format!("{} 不支援 WebSocket", provider_id))?;

    let sender = Arc::new(state.ws_sender.clone());
    let mut receiver = state.ws_sender.subscribe();

    // Start WS connection — returns the connection task handle
    let ws_handle = ws_provider.subscribe(symbols, sender).await?;

    // Forward WS updates to frontend via Tauri events
    let app_handle = app.clone();
    let forwarder = tokio::spawn(async move {
        while let Ok(update) = receiver.recv().await {
            let _ = app_handle.emit("ws-ticker-update", &update);
        }
    });

    let mut tasks = state.ws_tasks.write().await;
    tasks.insert(provider_id, (forwarder, ws_handle));

    Ok(())
}

/// Stop a WebSocket connection
#[tauri::command]
pub async fn stop_ws_stream(
    state: tauri::State<'_, AppState>,
    provider_id: String,
) -> Result<(), String> {
    let mut tasks = state.ws_tasks.write().await;
    if let Some((forwarder, ws_conn)) = tasks.remove(&provider_id) {
        forwarder.abort();
        ws_conn.abort();
    }
    Ok(())
}

/// Set custom icon for a subscription symbol
#[tauri::command]
pub async fn set_icon(app: tauri::AppHandle, symbol: String) -> Result<String, String> {
    use tauri::Manager;

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

    let app_dir = app.path().app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?;
    let icons_dir = app_dir.join("icons");
    tokio::fs::create_dir_all(&icons_dir).await
        .map_err(|e| format!("建立 icons 目錄失敗: {}", e))?;

    let dest = icons_dir.join(format!("{}.png", icon_name));
    let bytes = file.read().await;
    tokio::fs::write(&dest, &bytes).await
        .map_err(|e| format!("寫入圖示失敗: {}", e))?;

    Ok(dest.to_string_lossy().to_string())
}

/// Remove custom icon for a subscription symbol
#[tauri::command]
pub async fn remove_icon(app: tauri::AppHandle, symbol: String) -> Result<(), String> {
    use tauri::Manager;

    let icon_name = symbol
        .to_lowercase()
        .replace("usdt", "")
        .replace("-usd", "");

    let app_dir = app.path().app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?;
    let dest = app_dir.join("icons").join(format!("{}.png", icon_name));

    if dest.exists() {
        tokio::fs::remove_file(&dest).await
            .map_err(|e| format!("刪除圖示失敗: {}", e))?;
    }
    Ok(())
}

/// Get the app data icons directory path
#[tauri::command]
pub async fn get_icons_dir(app: tauri::AppHandle) -> Result<String, String> {
    use tauri::Manager;

    let app_dir = app.path().app_data_dir()
        .map_err(|e| format!("無法取得 app 目錄: {}", e))?;
    let icons_dir = app_dir.join("icons");
    Ok(icons_dir.to_string_lossy().to_string())
}

/// Export file with native save dialog
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

/// Import file with native open dialog
#[tauri::command]
pub async fn import_file() -> Result<String, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("JSON", &["json"])
        .pick_file()
        .await
        .ok_or_else(|| "已取消".to_string())?;

    let bytes = file.read().await;
    String::from_utf8(bytes).map_err(|e| format!("讀取失敗: {}", e))
}
