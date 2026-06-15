use crate::core_state::CoreState;
use std::sync::Arc;
use tauri_plugin_shell::ShellExt;

#[tauri::command]
pub async fn get_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    use tauri::Manager;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app directory: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        let path_str = dir.to_string_lossy().to_string();
        app.shell()
            .command("explorer")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        let path_str = dir.to_string_lossy().to_string();
        app.shell()
            .command("open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let path_str = dir.to_string_lossy().to_string();
        app.shell()
            .command("xdg-open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(dir.to_string_lossy().to_string())
}

// ── API Settings ────────────────────────────────────────────────

#[tauri::command]
pub async fn get_api_port(state: tauri::State<'_, Arc<CoreState>>) -> Result<u16, String> {
    let val = state.db.get_setting("api_port")?.unwrap_or("8080".into());
    val.parse::<u16>()
        .map_err(|e| format!("Invalid port: {}", e))
}

#[tauri::command]
pub async fn set_api_port(state: tauri::State<'_, Arc<CoreState>>, port: u16) -> Result<(), String> {
    if port < 1024 {
        return Err("Port must be between 1024 and 65535".to_string());
    }
    state.db.set_setting("api_port", &port.to_string())
}

#[tauri::command]
pub async fn get_api_enabled(state: tauri::State<'_, Arc<CoreState>>) -> Result<bool, String> {
    let val = state.db.get_setting("api_enabled")?.unwrap_or("0".into());
    Ok(val == "1")
}

#[tauri::command]
pub async fn set_api_enabled(
    state: tauri::State<'_, Arc<CoreState>>,
    enabled: bool,
) -> Result<(), String> {
    state
        .db
        .set_setting("api_enabled", if enabled { "1" } else { "0" })
}

// ── Polling ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn reload_polling(state: tauri::State<'_, Arc<CoreState>>) -> Result<(), String> {
    state.polling.reload();
    Ok(())
}

#[tauri::command]
pub async fn set_unattended_polling(
    state: tauri::State<'_, Arc<CoreState>>,
    enabled: bool,
) -> Result<(), String> {
    state.polling.set_unattended(enabled).await;
    Ok(())
}

#[tauri::command]
pub async fn get_unattended_polling(state: tauri::State<'_, Arc<CoreState>>) -> Result<bool, String> {
    Ok(state.polling.is_unattended().await)
}
