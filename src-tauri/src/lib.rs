mod api_server;
mod commands;
mod db;
mod events;
mod notifications;
mod polling;
mod providers;

use commands::{
    add_sub_to_view, add_subscription, add_subscriptions_batch, cleanup_history, create_view,
    create_notification_rule, delete_notification_channel, delete_notification_rule,
    delete_subscription_history, delete_view, enable_provider, export_data, export_file,
    fetch_asset_price, fetch_multiple_prices, get_all_providers, get_api_enabled, get_api_port,
    get_cached_prices, get_data_dir, get_history_stats, get_icons_dir,
    get_notification_history, get_poll_ticks,
    get_price_history, get_theme_bg_path, get_unattended_polling, get_view_sub_counts,
    get_view_subscription_ids, has_api_key, import_data, import_file, list_notification_channels,
    list_notification_rules, list_provider_settings,
    list_subscriptions, list_views, lookup_dex_pool, purge_all_history, read_local_file_base64,
    reload_polling, remove_icon, remove_sub_from_view, remove_subscription,
    remove_subscriptions, remove_theme_bg, rename_view, reset_all_data, save_notification_channel,
    save_theme_bg, set_api_enabled,
    set_api_port, set_icon, set_provider_record_hours, set_record_hours,
    set_unattended_polling, set_visible_subscriptions, start_ws_stream, stop_ws_stream,
    test_notification_channel, toggle_notification_rule, toggle_record, update_notification_rule,
    update_subscription, upsert_provider_settings, AppState,
};
use db::DbPool;
use events::AppEvent;
use providers::registry::ProviderRegistry;
use std::collections::HashSet;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::broadcast;

/// 確保 DB schema 一致 — 版本不同就刪除重建
fn ensure_clean_db(app_dir: &std::path::Path) {
    let db_path = app_dir.join("stockenboard.db");
    let marker = app_dir.join(".schema_v");
    const SCHEMA_VER: &str = "8"; // Bumped for push notifications tables
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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            // Provider / Fetch
            fetch_asset_price,
            fetch_multiple_prices,
            get_all_providers,
            enable_provider,
            // Polling
            reload_polling,
            set_unattended_polling,
            get_unattended_polling,
            set_visible_subscriptions,
            get_cached_prices,
            get_poll_ticks,
            // Subscriptions (NEW)
            list_subscriptions,
            add_subscription,
            add_subscriptions_batch,
            update_subscription,
            remove_subscription,
            remove_subscriptions,
            has_api_key,
            // Provider Settings (NEW)
            list_provider_settings,
            upsert_provider_settings,
            // Views (NEW)
            list_views,
            create_view,
            rename_view,
            delete_view,
            get_view_sub_counts,
            get_view_subscription_ids,
            add_sub_to_view,
            remove_sub_from_view,
            // WebSocket
            start_ws_stream,
            stop_ws_stream,
            // Icons
            set_icon,
            remove_icon,
            get_icons_dir,
            read_local_file_base64,
            // Theme
            save_theme_bg,
            remove_theme_bg,
            get_theme_bg_path,
            // Import/Export
            export_file,
            import_file,
            export_data,
            import_data,
            // DEX
            lookup_dex_pool,
            // History
            toggle_record,
            set_record_hours,
            set_provider_record_hours,
            get_price_history,
            get_history_stats,
            cleanup_history,
            purge_all_history,
            delete_subscription_history,
            reset_all_data,
            // Misc
            get_data_dir,
            get_api_port,
            set_api_port,
            get_api_enabled,
            set_api_enabled,
            // Notifications
            create_notification_rule,
            list_notification_rules,
            update_notification_rule,
            delete_notification_rule,
            toggle_notification_rule,
            save_notification_channel,
            list_notification_channels,
            delete_notification_channel,
            test_notification_channel,
            get_notification_history,
        ])
        .setup(|app| {
            if let Ok(app_dir) = app.path().app_data_dir() {
                ensure_clean_db(&app_dir);

                let db_path = app_dir.join("stockenboard.db");

                // 建立統一的 DbPool（WAL mode + busy_timeout）
                let db = Arc::new(
                    DbPool::open(&db_path).expect("無法開啟資料庫"),
                );

                // 建立共享 Provider Registry（含 rate limiting）
                let registry = Arc::new(ProviderRegistry::new());

                // 建立 Event Bus（解耦 Polling ↔ DB ↔ 前端）
                let (event_bus, _) = broadcast::channel::<AppEvent>(512);

                let state = AppState::new(db.clone(), registry.clone(), event_bus.clone());
                state.polling.start(
                    app.handle().clone(),
                    db.clone(),
                    registry.clone(),
                    event_bus.clone(),
                );

                // 啟動 Event Forwarder（將 AppEvent 轉發到前端 + DB）
                let db_for_forwarder = db.clone();
                let app_for_forwarder = app.handle().clone();
                let mut event_rx = event_bus.subscribe();
                tauri::async_runtime::spawn(async move {
                    loop {
                        match event_rx.recv().await {
                            Ok(event) => match event {
                                AppEvent::PriceUpdate {
                                    provider_id,
                                    data,
                                    record_symbols,
                                } => {
                                    // 轉發到前端
                                    let _ = app_for_forwarder.emit("price-update", &data);
                                    // 寫入 price_history
                                    if !record_symbols.is_empty() {
                                        let record_set: HashSet<String> =
                                            record_symbols.into_iter().collect();
                                        let records: Vec<(
                                            String,
                                            f64,
                                            Option<f64>,
                                            Option<f64>,
                                            Option<f64>,
                                            Option<f64>,
                                        )> = data
                                            .iter()
                                            .filter(|d| record_set.contains(&d.symbol))
                                            .map(|d| {
                                                let pre = d
                                                    .extra
                                                    .as_ref()
                                                    .and_then(|e| e.get("pre_market_price"))
                                                    .and_then(|v| v.as_f64());
                                                let post = d
                                                    .extra
                                                    .as_ref()
                                                    .and_then(|e| e.get("post_market_price"))
                                                    .and_then(|v| v.as_f64());
                                                (
                                                    d.symbol.clone(),
                                                    d.price,
                                                    d.change_percent_24h,
                                                    d.volume,
                                                    pre,
                                                    post,
                                                )
                                            })
                                            .collect();
                                        db_for_forwarder
                                            .write_price_history(&provider_id, &records);
                                    }
                                }
                                AppEvent::PriceError {
                                    provider_id,
                                    symbols,
                                    error,
                                } => {
                                    let payload: std::collections::HashMap<String, String> =
                                        symbols
                                            .iter()
                                            .map(|s| {
                                                (format!("{}:{}", provider_id, s), error.clone())
                                            })
                                            .collect();
                                    let _ = app_for_forwarder.emit("price-error", &payload);
                                }
                                AppEvent::PollTick {
                                    provider_id,
                                    fetched_at,
                                    interval_ms,
                                } => {
                                    let _ = app_for_forwarder.emit(
                                        "poll-tick",
                                        &events::PollTickPayload {
                                            provider_id,
                                            fetched_at,
                                            interval_ms,
                                        },
                                    );
                                }
                            },
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                eprintln!("[EventBus] Forwarder 落後 {} 事件", n);
                            }
                            Err(broadcast::error::RecvError::Closed) => break,
                        }
                    }
                });

                app.manage(state);

                // 啟動 Notification Engine
                let notification_engine = notifications::engine::NotificationEngine::new(db.clone());
                let notification_event_rx = event_bus.subscribe();
                tauri::async_runtime::spawn(async move {
                    notification_engine.reload_rules().await;
                    notification_engine.start(notification_event_rx);
                });

                // 啟動 API Server
                let app_handle = app.handle().clone();
                let db_for_api = db.clone();
                tauri::async_runtime::spawn(async move {
                    let enabled = db_for_api
                        .get_setting("api_enabled")
                        .ok()
                        .flatten()
                        .map(|s| s == "1")
                        .unwrap_or(false);

                    let port = db_for_api
                        .get_setting("api_port")
                        .ok()
                        .flatten()
                        .and_then(|s| s.parse::<u16>().ok())
                        .unwrap_or(8080);

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
