use crate::core_state::CoreState;
use crate::db::{ViewRow, ViewSubCount};
use std::sync::Arc;

#[tauri::command]
pub async fn list_views(
    state: tauri::State<'_, Arc<CoreState>>,
    view_type: String,
) -> Result<Vec<ViewRow>, String> {
    state.db.list_views(&view_type)
}

#[tauri::command]
pub async fn create_view(
    state: tauri::State<'_, Arc<CoreState>>,
    name: String,
    view_type: String,
) -> Result<i64, String> {
    state.db.create_view(&name, &view_type)
}

#[tauri::command]
pub async fn rename_view(
    state: tauri::State<'_, Arc<CoreState>>,
    id: i64,
    name: String,
) -> Result<(), String> {
    state.db.rename_view(id, &name)
}

#[tauri::command]
pub async fn delete_view(state: tauri::State<'_, Arc<CoreState>>, id: i64) -> Result<(), String> {
    state.db.delete_view(id)
}

#[tauri::command]
pub async fn get_view_sub_counts(
    state: tauri::State<'_, Arc<CoreState>>,
) -> Result<Vec<ViewSubCount>, String> {
    state.db.get_view_sub_counts()
}

#[tauri::command]
pub async fn get_view_subscription_ids(
    state: tauri::State<'_, Arc<CoreState>>,
    view_id: i64,
) -> Result<Vec<i64>, String> {
    state.db.get_view_subscription_ids(view_id)
}

#[tauri::command]
pub async fn add_sub_to_view(
    state: tauri::State<'_, Arc<CoreState>>,
    view_id: i64,
    subscription_id: i64,
) -> Result<(), String> {
    state.db.add_sub_to_view(view_id, subscription_id)
}

#[tauri::command]
pub async fn remove_sub_from_view(
    state: tauri::State<'_, Arc<CoreState>>,
    view_id: i64,
    subscription_id: i64,
) -> Result<(), String> {
    state.db.remove_sub_from_view(view_id, subscription_id)
}
