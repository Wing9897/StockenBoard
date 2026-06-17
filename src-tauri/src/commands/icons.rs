use crate::core_state::CoreState;
use std::sync::Arc;
use tauri::Manager;

/// Opens the icons directory in the native file explorer.
/// Creates the directory if it does not exist.
#[tauri::command]
pub async fn open_icons_folder(
    state: tauri::State<'_, Arc<CoreState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let icons_dir = state.data_dir.join("icons");

    // Ensure directory exists
    tokio::fs::create_dir_all(&icons_dir)
        .await
        .map_err(|e| format!("Failed to create icons directory: {}", e))?;

    // Use tauri-plugin-opener to open the directory in the file manager
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(icons_dir.to_string_lossy().as_ref(), None::<&str>)
        .map_err(|e| format!("Failed to open folder: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn set_icon(
    state: tauri::State<'_, Arc<CoreState>>,
    symbol: String,
) -> Result<String, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "svg"])
        .set_title("Select Icon")
        .pick_file()
        .await
        .ok_or_else(|| "Cancelled".to_string())?;
    let icon_name = symbol.to_lowercase();
    let icons_dir = state.data_dir.join("icons");
    tokio::fs::create_dir_all(&icons_dir)
        .await
        .map_err(|e| format!("Failed to create icons directory: {}", e))?;
    let dest = icons_dir.join(format!("{}.png", icon_name));
    tokio::fs::write(&dest, file.read().await)
        .await
        .map_err(|e| format!("Failed to write icon: {}", e))?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn remove_icon(
    state: tauri::State<'_, Arc<CoreState>>,
    symbol: String,
) -> Result<(), String> {
    let icon_name = symbol.to_lowercase();
    let dest = state.data_dir.join("icons").join(format!("{}.png", icon_name));
    if dest.exists() {
        tokio::fs::remove_file(&dest)
            .await
            .map_err(|e| format!("Failed to delete icon: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_icons_dir(
    state: tauri::State<'_, Arc<CoreState>>,
) -> Result<String, String> {
    let dir = state.data_dir.join("icons");
    Ok(dir.to_string_lossy().to_string())
}

// ── Logo Batch Download ─────────────────────────────────────────

pub use crate::icons::LogoDownloadResult;

/// Batch-download subscription logos (delegates to shared icons module).
#[tauri::command]
pub async fn download_logos(
    state: tauri::State<'_, Arc<CoreState>>,
    app: tauri::AppHandle,
) -> Result<LogoDownloadResult, String> {
    use tauri::Emitter;

    let icons_dir = state.data_dir.join("icons");

    // Set up a progress channel to forward events to the Tauri frontend
    let (progress_tx, mut progress_rx) =
        tokio::sync::broadcast::channel::<crate::icons::DownloadProgress>(64);

    let app_handle = app.clone();
    tokio::spawn(async move {
        while let Ok(progress) = progress_rx.recv().await {
            let _ = app_handle.emit(
                "logo-download-progress",
                serde_json::json!({
                    "current": progress.current, "total": progress.total, "symbol": progress.symbol
                }),
            );
        }
    });

    crate::icons::download_all_logos(&state.db, &icons_dir, Some(progress_tx)).await
}

#[tauri::command]
pub async fn clear_all_icons(
    state: tauri::State<'_, Arc<CoreState>>,
) -> Result<i64, String> {
    let icons_dir = state.data_dir.join("icons");
    if !icons_dir.exists() {
        return Ok(0);
    }
    let mut count: i64 = 0;
    let mut entries = tokio::fs::read_dir(&icons_dir)
        .await
        .map_err(|e| format!("Failed to read icons directory: {}", e))?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().map(|e| e == "png").unwrap_or(false) {
            if tokio::fs::remove_file(&path).await.is_ok() {
                count += 1;
            }
        }
    }
    Ok(count)
}

#[tauri::command]
pub async fn download_single_icon(
    state: tauri::State<'_, Arc<CoreState>>,
    symbol: String,
    save_as: String,
) -> Result<(), String> {
    let icons_dir = state.data_dir.join("icons");
    tokio::fs::create_dir_all(&icons_dir)
        .await
        .map_err(|e| format!("Failed to create icons directory: {}", e))?;

    let bytes = crate::icons::try_download_png(
        &reqwest::Client::new(),
        &symbol,
        false,
    ).await.ok_or_else(|| format!("Logo not found for symbol: {}", symbol))?;

    let dest = icons_dir.join(format!("{}.png", save_as.to_lowercase()));
    tokio::fs::write(&dest, &bytes)
        .await
        .map_err(|e| format!("Failed to save icon: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn search_icons(
    symbol: String,
) -> Result<Vec<crate::icons::IconSearchResult>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("StockenBoard/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    Ok(crate::icons::search_icons(&client, &symbol).await)
}

#[tauri::command]
pub async fn save_icon_from_data(
    state: tauri::State<'_, Arc<CoreState>>,
    save_as: String,
    data_url: String,
) -> Result<(), String> {
    use base64::Engine;
    let icons_dir = state.data_dir.join("icons");
    tokio::fs::create_dir_all(&icons_dir)
        .await
        .map_err(|e| format!("Failed to create icons directory: {}", e))?;

    // Parse data URL: "data:image/png;base64,{base64}"
    let b64_part = data_url
        .split(",")
        .nth(1)
        .ok_or_else(|| "Invalid data URL format".to_string())?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64_part)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    let dest = icons_dir.join(format!("{}.png", save_as.to_lowercase()));
    tokio::fs::write(&dest, &bytes)
        .await
        .map_err(|e| format!("Failed to save icon: {}", e))?;
    Ok(())
}

/// 讀取本地檔案並回傳 base64 data URL
#[tauri::command]
pub async fn read_local_file_base64(path: String) -> Result<String, String> {
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;
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
        .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
        .set_title("Select Background Image")
        .pick_file()
        .await
        .ok_or_else(|| "Cancelled".to_string())?;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app directory: {}", e))?
        .join("theme_bg");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("Failed to create directory: {}", e))?;
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
        .map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn remove_theme_bg(app: tauri::AppHandle, theme_id: String) -> Result<(), String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app directory: {}", e))?
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
        .map_err(|e| format!("Failed to get app directory: {}", e))?
        .join("theme_bg");
    for ext in &["png", "jpg", "jpeg", "webp", "img"] {
        let path = dir.join(format!("{}.{}", theme_id, ext));
        if path.exists() {
            return Ok(Some(path.to_string_lossy().to_string()));
        }
    }
    Ok(None)
}
