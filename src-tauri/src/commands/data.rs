use crate::core_state::CoreState;
use crate::db::ExportData;
use std::sync::Arc;

#[tauri::command]
pub async fn export_file(filename: String, content: String) -> Result<(), String> {
    let path = rfd::AsyncFileDialog::new()
        .set_file_name(&filename)
        .add_filter("JSON", &["json"])
        .save_file()
        .await
        .ok_or_else(|| "Cancelled".to_string())?;
    tokio::fs::write(path.path(), content.as_bytes())
        .await
        .map_err(|e| format!("Write failed: {}", e))
}

#[tauri::command]
pub async fn import_file() -> Result<String, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("JSON", &["json"])
        .pick_file()
        .await
        .ok_or_else(|| "Cancelled".to_string())?;
    String::from_utf8(file.read().await).map_err(|e| format!("Read failed: {}", e))
}

#[tauri::command]
pub async fn export_data(state: tauri::State<'_, Arc<CoreState>>) -> Result<ExportData, String> {
    state.db.export_data()
}

#[tauri::command]
pub async fn import_data(
    state: tauri::State<'_, Arc<CoreState>>,
    data: ExportData,
) -> Result<(usize, usize), String> {
    let result = state.db.import_data(&data)?;
    state.polling.reload();
    Ok(result)
}

#[tauri::command]
pub async fn reset_all_data(state: tauri::State<'_, Arc<CoreState>>) -> Result<(), String> {
    state.db.reset_all_data()?;
    state.notification_engine.reload_rules().await;
    state.polling.reload();
    Ok(())
}

// ── Price History ────────────────────────────────────────────────

#[tauri::command]
pub async fn toggle_record(
    state: tauri::State<'_, Arc<CoreState>>,
    subscription_id: i64,
    enabled: bool,
) -> Result<(), String> {
    state.db.toggle_record(subscription_id, enabled)?;
    state.polling.reload();
    Ok(())
}

#[tauri::command]
pub async fn set_record_hours(
    state: tauri::State<'_, Arc<CoreState>>,
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
    state: tauri::State<'_, Arc<CoreState>>,
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
    state: tauri::State<'_, Arc<CoreState>>,
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
    state: tauri::State<'_, Arc<CoreState>>,
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
    state: tauri::State<'_, Arc<CoreState>>,
    retention_days: Option<i64>,
) -> Result<i64, String> {
    let days = retention_days.unwrap_or(90);
    let cutoff = chrono::Utc::now().timestamp() - (days * 86400);
    state.db.cleanup_history(cutoff)
}

#[tauri::command]
pub async fn purge_all_history(state: tauri::State<'_, Arc<CoreState>>) -> Result<(), String> {
    state.db.purge_all_history()
}

#[tauri::command]
pub async fn delete_subscription_history(
    state: tauri::State<'_, Arc<CoreState>>,
    subscription_id: i64,
) -> Result<i64, String> {
    state.db.delete_history_for_subscription(subscription_id)
}
