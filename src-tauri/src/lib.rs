mod commands;
mod db;
mod polling;
mod providers;

use commands::{
    enable_provider, export_file, fetch_asset_price, fetch_multiple_prices,
    get_all_providers, get_cached_prices, get_icons_dir, get_poll_ticks,
    get_theme_bg_path, get_unattended_polling, import_file, lookup_dex_pool,
    read_local_file_base64, reload_polling, remove_icon, remove_theme_bg,
    save_theme_bg, set_icon, set_unattended_polling, set_visible_subscriptions,
    start_ws_stream, stop_ws_stream, toggle_record, set_record_hours,
    set_provider_record_hours, get_price_history,
    get_history_stats, cleanup_history, purge_all_history, get_data_dir, AppState,
};
use tauri::Manager;
use tauri_plugin_sql::{Migration, MigrationKind};

/// 確保 DB schema 一致 — 版本不同就刪除重建
fn ensure_clean_db(app_dir: &std::path::Path) {
    let db_path = app_dir.join("stockenboard.db");
    let marker = app_dir.join(".schema_v");
    const SCHEMA_VER: &str = "5";
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
        version: 5,
        description: "initial_schema",
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
