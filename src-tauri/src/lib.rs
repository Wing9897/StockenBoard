mod commands;
mod db;
mod polling;
mod providers;

use commands::{
    disable_provider, enable_provider, export_file, fetch_asset_price, fetch_multiple_prices,
    get_all_providers, get_cached_prices, get_icons_dir, get_poll_ticks, import_file,
    lookup_dex_pool, reload_polling, remove_icon, set_icon, set_visible_subscriptions,
    start_ws_stream, stop_ws_stream, AppState,
};
use tauri::Manager;
use tauri_plugin_sql::{Migration, MigrationKind};

/// 刪除舊版 DB（migration checksum 不兼容時自動重建）
fn ensure_clean_db(app_dir: &std::path::Path) {
    let db_path = app_dir.join("stockenboard.db");
    // 標記檔：記錄目前 schema 版本，版本不同就刪 DB 重建
    let marker = app_dir.join(".schema_v");
    const SCHEMA_VER: &str = "2";
    let current = std::fs::read_to_string(&marker).unwrap_or_default();
    if current.trim() != SCHEMA_VER {
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
        version: 2,
        description: "unified_schema_v2",
        sql: db::SCHEMA,
        kind: MigrationKind::Up,
    }];

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
            disable_provider,
            start_ws_stream,
            stop_ws_stream,
            export_file,
            import_file,
            set_icon,
            remove_icon,
            get_icons_dir,
            reload_polling,
            lookup_dex_pool,
            get_cached_prices,
            get_poll_ticks,
            set_visible_subscriptions,
        ])
        .setup(|app| {
            if let Ok(app_dir) = app.path().app_data_dir() {
                ensure_clean_db(&app_dir);
                let db_path = app_dir.join("stockenboard.db");
                let state = app.state::<AppState>();
                state.set_db_path(db_path.clone());
                state.polling.start(app.handle().clone(), db_path);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
