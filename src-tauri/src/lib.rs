mod api_server;
mod commands;
mod db;
mod polling;
mod providers;

use commands::{
    cleanup_history, enable_provider, export_file, fetch_asset_price, fetch_multiple_prices,
    get_all_providers, get_api_enabled, get_api_port, get_cached_prices, get_data_dir,
    get_history_stats, get_icons_dir, get_poll_ticks, get_price_history, get_theme_bg_path,
    get_unattended_polling, import_file, lookup_dex_pool, purge_all_history,
    read_local_file_base64, reload_polling, remove_icon, remove_theme_bg, save_theme_bg,
    set_api_enabled, set_api_port, set_icon, set_provider_record_hours, set_record_hours,
    set_unattended_polling, set_visible_subscriptions, start_ws_stream, stop_ws_stream,
    toggle_record, AppState,
};
use tauri::Manager;
use tauri_plugin_sql::{Migration, MigrationKind};

/// 確保 DB schema 一致 — 版本不同就刪除重建
///
/// ⚠️  WARNING: 此函式在 schema 版本不符時會【刪除整個資料庫】再重建。
/// 這在開發階段是可接受的快速迭代策略，但正式發佈前【必須】改為增量遷移
/// (incremental migration)。`tauri_plugin_sql` 已原生支援多版本遷移，
/// 只需在 `run()` 的 `migrations` vec 中逐步新增 Migration 即可。
///
/// TODO(release): 改為增量遷移，避免使用者資料遺失。
fn ensure_clean_db(app_dir: &std::path::Path) {
    let db_path = app_dir.join("stockenboard.db");
    let marker = app_dir.join(".schema_v");
    const SCHEMA_VER: &str = "6";
    let current = std::fs::read_to_string(&marker).unwrap_or_default();
    if current.trim() != SCHEMA_VER {
        eprintln!(
            "[DB] Schema 版本不符 (current={:?}, expected={}), 刪除並重建資料庫",
            current.trim(),
            SCHEMA_VER
        );
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
        let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
        let _ = std::fs::create_dir_all(app_dir);
        let _ = std::fs::write(&marker, SCHEMA_VER);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![Migration {
        version: 6,
        description: "initial_schema",
        sql: db::SCHEMA,
        kind: MigrationKind::Up,
    }];

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:stockenboard.db", migrations)
                .build(),
        )
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            fetch_asset_price,
            fetch_multiple_prices,
            get_all_providers,
            enable_provider,
            start_ws_stream,
            stop_ws_stream,
            export_file,
            import_file,
            set_icon,
            remove_icon,
            get_icons_dir,
            read_local_file_base64,
            reload_polling,
            lookup_dex_pool,
            get_cached_prices,
            get_poll_ticks,
            set_visible_subscriptions,
            save_theme_bg,
            remove_theme_bg,
            get_theme_bg_path,
            set_unattended_polling,
            get_unattended_polling,
            toggle_record,
            set_record_hours,
            set_provider_record_hours,
            get_price_history,
            get_history_stats,
            cleanup_history,
            purge_all_history,
            get_data_dir,
            get_api_port,
            set_api_port,
            get_api_enabled,
            set_api_enabled,
        ])
        .setup(|app| {
            if let Ok(app_dir) = app.path().app_data_dir() {
                ensure_clean_db(&app_dir);
                let db_path = app_dir.join("stockenboard.db");
                let state = app.state::<AppState>();
                state.set_db_path(db_path.clone());
                state.polling.start(app.handle().clone(), db_path.clone());

                // 啟動 API Server（從 DB 讀取 enabled 和 port）
                let app_handle = app.handle().clone();
                let db_path_for_api = db_path.clone();
                tauri::async_runtime::spawn(async move {
                    // 從 DB 讀取 API 設定
                    let (enabled, port) = match rusqlite::Connection::open(&db_path_for_api) {
                        Ok(conn) => {
                            let enabled = conn
                                .query_row(
                                    "SELECT value FROM app_settings WHERE key = 'api_enabled'",
                                    [],
                                    |row| row.get::<_, String>(0),
                                )
                                .ok()
                                .map(|s| s == "1")
                                .unwrap_or(false);

                            let port = conn
                                .query_row(
                                    "SELECT value FROM app_settings WHERE key = 'api_port'",
                                    [],
                                    |row| row.get::<_, String>(0),
                                )
                                .ok()
                                .and_then(|s| s.parse::<u16>().ok())
                                .unwrap_or(8080);

                            (enabled, port)
                        }
                        Err(_) => (false, 8080),
                    };

                    if !enabled {
                        println!("[API] Server 已停用");
                        return;
                    }

                    let state: tauri::State<AppState> = app_handle.state();
                    let state_arc = std::sync::Arc::new(state.clone_for_api());
                    if let Err(e) = api_server::start_api_server(state_arc, port).await {
                        eprintln!("[API] Server 啟動失敗: {}", e);
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
