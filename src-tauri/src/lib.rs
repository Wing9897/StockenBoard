mod commands;
mod db;
mod polling;
mod providers;

use commands::{
    disable_provider, enable_provider, export_file, fetch_asset_price, fetch_multiple_prices,
    get_all_providers, get_cached_prices, get_icons_dir, get_poll_ticks, import_file,
    reload_polling, remove_icon, set_icon, set_visible_subscriptions, start_ws_stream,
    stop_ws_stream, AppState,
};
use tauri::Manager;
use tauri_plugin_sql::{Migration, MigrationKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![Migration {
        version: 1,
        description: "create_tables_v2",
        sql: db::MIGRATION_V1,
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
            get_cached_prices,
            get_poll_ticks,
            set_visible_subscriptions,
        ])
        .setup(|app| {
            if let Ok(app_dir) = app.path().app_data_dir() {
                let db_path = app_dir.join("stockenboard.db");
                app.state::<AppState>().polling.start(app.handle().clone(), db_path);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
