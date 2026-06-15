use crate::core_state::CoreState;
use crate::db::ProviderSettingsRow;
use std::sync::Arc;

#[tauri::command]
pub async fn list_provider_settings(
    state: tauri::State<'_, Arc<CoreState>>,
) -> Result<Vec<ProviderSettingsRow>, String> {
    state.db.list_provider_settings()
}

#[tauri::command]
// 引數對應前端 IPC 契約與 provider_settings 資料表欄位，刻意保持平面簽章
#[allow(clippy::too_many_arguments)]
pub async fn upsert_provider_settings(
    state: tauri::State<'_, Arc<CoreState>>,
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
