mod commands;
mod db;
mod providers;

use commands::{
    fetch_asset_price, fetch_multiple_prices, get_all_providers,
    enable_provider, disable_provider, start_ws_stream, stop_ws_stream,
    export_file, import_file, set_icon, remove_icon, get_icons_dir,
    AppState,
};
use tauri::Manager;
use tauri_plugin_sql::{Migration, MigrationKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![
        Migration {
            version: 1,
            description: "create_tables",
            sql: db::MIGRATION_V1,
            kind: MigrationKind::Up,
        },
    ];

    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:stockenboard.db", migrations)
                .build(),
        )
        .manage(app_state)
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
        ])
        .setup(|app| {
            // 從 DB 讀取已儲存的 API key，確保重啟後 key 仍然生效
            if let Ok(app_dir) = app.path().app_data_dir() {
                let state = app.state::<AppState>();
                state.init_from_db_sync(&app_dir);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
