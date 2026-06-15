use crate::core_state::CoreState;
use crate::polling::PollTick;
use crate::providers::{
    create_dex_lookup, create_ws_provider, get_all_provider_info, AssetData, DexPoolInfo,
    ProviderInfo,
};
use std::sync::Arc;
use tauri::Emitter;

#[tauri::command]
pub async fn fetch_asset_price(
    state: tauri::State<'_, Arc<CoreState>>,
    provider_id: String,
    symbol: String,
) -> Result<AssetData, String> {
    let p = state
        .registry
        .get_or_create(&provider_id, &state.db)
        .await
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;
    p.fetch_price(&symbol).await
}

#[tauri::command]
pub async fn fetch_multiple_prices(
    state: tauri::State<'_, Arc<CoreState>>,
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
    state: tauri::State<'_, Arc<CoreState>>,
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
pub async fn get_cached_prices(
    state: tauri::State<'_, Arc<CoreState>>,
) -> Result<Vec<AssetData>, String> {
    Ok(state.polling.cache.read().await.values().cloned().collect())
}

#[tauri::command]
pub async fn get_poll_ticks(state: tauri::State<'_, Arc<CoreState>>) -> Result<Vec<PollTick>, String> {
    Ok(state.polling.ticks.read().await.values().cloned().collect())
}

#[tauri::command]
pub async fn set_visible_subscriptions(
    state: tauri::State<'_, Arc<CoreState>>,
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
    state: tauri::State<'_, Arc<CoreState>>,
    provider_id: String,
    pool_address: String,
) -> Result<DexPoolInfo, String> {
    let settings = state.db.get_provider_settings(&provider_id).ok().flatten();
    let api_key = settings.as_ref().and_then(|s| s.api_key.clone());
    let api_url = settings.as_ref().and_then(|s| s.api_url.clone());
    let lookup = create_dex_lookup(&provider_id, api_key, api_url)
        .ok_or_else(|| format!("{} does not support pool lookup", provider_id))?;
    lookup.lookup_pool(&pool_address).await
}

// ── WebSocket ───────────────────────────────────────────────────

#[tauri::command]
pub async fn start_ws_stream(
    state: tauri::State<'_, Arc<CoreState>>,
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
        .ok_or_else(|| format!("{} does not support WebSocket", provider_id))?;
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
    state: tauri::State<'_, Arc<CoreState>>,
    provider_id: String,
) -> Result<(), String> {
    if let Some((fwd, ws)) = state.ws_tasks.write().await.remove(&provider_id) {
        fwd.abort();
        ws.abort();
    }
    Ok(())
}
