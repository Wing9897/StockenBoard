use crate::core_state::CoreState;
use crate::db::{BatchAddItem, BatchAddResult, Subscription};
use std::sync::Arc;

#[tauri::command]
pub async fn list_subscriptions(
    state: tauri::State<'_, Arc<CoreState>>,
    sub_type: String,
) -> Result<Vec<Subscription>, String> {
    state.db.list_subscriptions(&sub_type)
}

#[tauri::command]
pub async fn list_all_subscriptions(
    state: tauri::State<'_, Arc<CoreState>>,
) -> Result<Vec<Subscription>, String> {
    state.db.list_all_subscriptions()
}

#[tauri::command]
// 引數對應前端 IPC 契約與 subscriptions 資料表欄位，刻意保持平面簽章（見 Requirement 6.4）
#[allow(clippy::too_many_arguments)]
pub async fn add_subscription(
    state: tauri::State<'_, Arc<CoreState>>,
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

#[tauri::command]
pub async fn add_subscriptions_batch(
    state: tauri::State<'_, Arc<CoreState>>,
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
            Err(e) if e.contains("already exists") => duplicates.push(normalized),
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
    state: tauri::State<'_, Arc<CoreState>>,
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
pub async fn remove_subscription(state: tauri::State<'_, Arc<CoreState>>, id: i64) -> Result<(), String> {
    state.db.remove_subscription(id)?;
    state.polling.reload();
    Ok(())
}

#[tauri::command]
pub async fn remove_subscriptions(
    state: tauri::State<'_, Arc<CoreState>>,
    ids: Vec<i64>,
) -> Result<(), String> {
    state.db.remove_subscriptions(&ids)?;
    state.polling.reload();
    Ok(())
}

#[tauri::command]
pub async fn has_api_key(
    state: tauri::State<'_, Arc<CoreState>>,
    provider_id: String,
) -> Result<bool, String> {
    Ok(state.db.has_api_key(&provider_id))
}
