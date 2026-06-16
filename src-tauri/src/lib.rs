pub mod api;
#[cfg(feature = "desktop")]
mod commands;
pub mod config;
pub mod core_state;
pub mod db;
pub mod events;
pub mod icons;
pub mod notifications;
pub mod polling;
pub mod providers;

#[cfg(feature = "desktop")]
use commands::{
    add_sub_to_view, add_subscription, add_subscriptions_batch, cleanup_history,
    create_notification_rule, create_view, delete_notification_channel, delete_notification_rule,
    delete_subscription_history, delete_view, download_logos, enable_provider, export_data,
    export_file, fetch_asset_price, fetch_multiple_prices, get_ai_provider_config, get_all_providers,
    get_api_enabled, get_api_port, get_cached_prices, get_data_dir, get_history_stats,
    get_icons_dir, get_notification_global_cooldown, get_notification_history, get_poll_ticks, open_icons_folder,
    get_price_history, get_theme_bg_path, get_unattended_polling, get_view_sub_counts,
    get_view_subscription_ids, has_api_key, import_data, import_file, list_all_subscriptions,
    list_notification_channels, list_notification_rules,
    list_provider_settings, list_subscriptions, list_views, lookup_dex_pool, purge_all_history,
    read_local_file_base64, reload_polling, remove_icon, remove_sub_from_view, remove_subscription,
    remove_subscriptions, remove_theme_bg, rename_view, reset_all_data, save_ai_provider_config,
    save_notification_channel, save_theme_bg, set_api_enabled, set_api_port, set_icon,
    set_notification_global_cooldown, set_provider_record_hours, set_record_hours,
    set_unattended_polling, set_visible_subscriptions, start_ws_stream, stop_ws_stream,
    test_ai_connection, list_ai_models, test_notification_channel, toggle_notification_rule,
    toggle_record, update_notification_rule, update_subscription, upsert_provider_settings,
};

#[cfg(feature = "desktop")]
use core_state::CoreState;
#[cfg(feature = "desktop")]
use db::PriceRecord;
#[cfg(feature = "desktop")]
use events::AppEvent;
#[cfg(feature = "desktop")]
use std::collections::HashSet;
#[cfg(feature = "desktop")]
use std::sync::Arc;
#[cfg(feature = "desktop")]
use tauri::{Emitter, Manager};
#[cfg(feature = "desktop")]
use tokio::sync::broadcast;

#[cfg(feature = "desktop")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            // 注意：部分指令目前前端尚未呼叫（如 enable_provider、get_unattended_polling、
            // remove_icon、set_provider_record_hours、get_history_stats），屬刻意保留的 IPC 介面：
            // 其底層能力已存在且部分經由其他路徑使用（例：api/ module 直接用 polling.is_unattended()，
            // record hours 經由 upsert_provider_settings 寫入）。保留以供未來 UI／HTTP API 擴充，
            // 移除前請先確認無外部依賴。
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
            list_all_subscriptions,
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
            open_icons_folder,
            download_logos,
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
            get_notification_global_cooldown,
            set_notification_global_cooldown,
            // AI Provider Config
            save_ai_provider_config,
            get_ai_provider_config,
            test_ai_connection,
            list_ai_models,
        ])
        .setup(|app| {
            // Portable data directory: use SB_DATA_DIR env var if set,
            // otherwise fall back to `./data` relative to current working directory.
            // This ensures the same path in dev mode, release build, and server mode.
            let data_dir = std::env::var("SB_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("./data"));

            {
                // Build unified CoreState (handles DB, registry, event bus, etc.)
                let core = CoreState::new(&data_dir)
                    .expect("Failed to initialize CoreState");
                let core = Arc::new(core);

                // Start polling inside async context so tokio::spawn works
                let polling_ref = core.polling.clone();
                let db_for_polling = core.db.clone();
                let db_for_unattended = core.db.clone();
                let registry_for_polling = core.registry.clone();
                let event_bus_for_polling = core.event_bus.clone();
                tauri::async_runtime::spawn(async move {
                    // Auto-set unattended based on active recordings at startup
                    let active_count = db_for_unattended.count_active_recordings().unwrap_or(0);
                    if active_count > 0 {
                        polling_ref.set_unattended(true).await;
                    }

                    polling_ref.start(
                        db_for_polling,
                        registry_for_polling,
                        event_bus_for_polling,
                    );
                });

                let db_for_forwarder = core.db.clone();
                let app_for_forwarder = app.handle().clone();
                let mut event_rx = core.event_bus.subscribe();
                tauri::async_runtime::spawn(async move {
                    loop {
                        match event_rx.recv().await {
                            Ok(event) => match event {
                                AppEvent::PriceUpdate {
                                    provider_id,
                                    data,
                                    record_symbols,
                                } => {
                                    let _ = app_for_forwarder.emit("price-update", &data);
                                    if !record_symbols.is_empty() {
                                        let record_set: HashSet<String> =
                                            record_symbols.into_iter().collect();
                                        let records: Vec<PriceRecord> = data
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
                                AppEvent::NotificationTriggered(payload) => {
                                    let _ = app_for_forwarder
                                        .emit("notification-triggered", &payload);
                                }
                                AppEvent::SystemNotification { title, body } => {
                                    use tauri_plugin_notification::NotificationExt;
                                    let _ = app_for_forwarder
                                        .notification()
                                        .builder()
                                        .title(&title)
                                        .body(&body)
                                        .show();
                                }
                                AppEvent::LogoDownloadProgress(progress) => {
                                    let _ = app_for_forwarder
                                        .emit("logo-download-progress", &progress);
                                }
                            },
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                eprintln!("[EventBus] Forwarder lagged {} events", n);
                            }
                            Err(broadcast::error::RecvError::Closed) => break,
                        }
                    }
                });

                app.manage(core.clone());

                let engine_for_start = core.notification_engine.clone();
                let notification_event_rx = core.event_bus.subscribe();
                tauri::async_runtime::spawn(async move {
                    engine_for_start.reload_rules().await;
                    engine_for_start.start(notification_event_rx);
                });

                let ai_scheduler_for_start = core.ai_scheduler.clone();
                tauri::async_runtime::spawn(async move {
                    ai_scheduler_for_start.start().await;
                });

                let core_for_api = core.clone();
                tauri::async_runtime::spawn(async move {
                    let enabled = core_for_api
                        .db
                        .get_setting("api_enabled")
                        .ok()
                        .flatten()
                        .map(|s| s == "1")
                        .unwrap_or(false);

                    let port = core_for_api
                        .db
                        .get_setting("api_port")
                        .ok()
                        .flatten()
                        .and_then(|s| s.parse::<u16>().ok())
                        .unwrap_or(8080);

                    if !enabled {
                        eprintln!("[API] Server disabled");
                        return;
                    }

                    let app = api::build_router(core_for_api);
                    let addr = format!("127.0.0.1:{}", port);
                    eprintln!("[API] Starting HTTP server on http://{}", addr);

                    let listener = match tokio::net::TcpListener::bind(&addr).await {
                        Ok(l) => l,
                        Err(e) => {
                            eprintln!("[API] Failed to bind to {}: {}", addr, e);
                            return;
                        }
                    };

                    if let Err(e) = axum::serve(listener, app).await {
                        eprintln!("[API] Server error: {}", e);
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
