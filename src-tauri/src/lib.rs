pub mod api;
#[cfg(feature = "desktop")]
mod api_server;
#[cfg(feature = "desktop")]
mod commands;
pub mod config;
pub mod core_state;
pub mod db;
pub mod events;
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
    get_icons_dir, get_notification_global_cooldown, get_notification_history, get_poll_ticks,
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
    AppState,
};

#[cfg(feature = "desktop")]
use db::{DbPool, PriceRecord};
#[cfg(feature = "desktop")]
use events::AppEvent;
#[cfg(feature = "desktop")]
use notifications::global_cooldown::GlobalCooldown;
#[cfg(feature = "desktop")]
use providers::registry::ProviderRegistry;
#[cfg(feature = "desktop")]
use std::collections::HashSet;
#[cfg(feature = "desktop")]
use std::sync::Arc;
#[cfg(feature = "desktop")]
use tauri::{Emitter, Manager};
#[cfg(feature = "desktop")]
use tokio::sync::broadcast;

/// 確保 DB schema 一致 — 版本不同就刪除重建
/// (保留以維持 desktop 路徑相容性，底層委派給 core_state::ensure_clean_db)
#[cfg(feature = "desktop")]
fn ensure_clean_db(app_dir: &std::path::Path) {
    core_state::ensure_clean_db(app_dir);
}

#[cfg(feature = "desktop")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            // 注意：部分指令目前前端尚未呼叫（如 enable_provider、get_unattended_polling、
            // remove_icon、set_provider_record_hours、get_history_stats），屬刻意保留的 IPC 介面：
            // 其底層能力已存在且部分經由其他路徑使用（例：api_server 直接用 polling.is_unattended()，
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
            if let Ok(app_dir) = app.path().app_data_dir() {
                ensure_clean_db(&app_dir);

                let db_path = app_dir.join("stockenboard.db");

                // 建立統一的 DbPool（WAL mode + busy_timeout）
                let db = Arc::new(DbPool::open(&db_path).expect("無法開啟資料庫"));

                // 建立共享 Provider Registry（含 rate limiting）
                let registry = Arc::new(ProviderRegistry::new());

                // 建立 Event Bus（解耦 Polling ↔ DB ↔ 前端）
                let (event_bus, _) = broadcast::channel::<AppEvent>(512);

                // 建立 Global Cooldown（從 DB 讀取設定值，預設 30 秒）
                let cooldown_secs: u64 = db
                    .get_setting("notification_global_cooldown")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30);
                let global_cooldown = Arc::new(GlobalCooldown::new(cooldown_secs));

                // 建立 Notification Engine
                let notification_engine = Arc::new(
                    notifications::engine::NotificationEngine::new(db.clone(), event_bus.clone(), global_cooldown.clone()),
                );

                // 建立 AI Scheduler
                let ai_scheduler = Arc::new(
                    notifications::ai_scheduler::AiScheduler::new(db.clone())
                        .with_event_bus(event_bus.clone())
                        .with_global_cooldown(global_cooldown.clone()),
                );

                let state = AppState::new(
                    db.clone(),
                    registry.clone(),
                    event_bus.clone(),
                    notification_engine.clone(),
                    ai_scheduler.clone(),
                    global_cooldown.clone(),
                );

                // Start polling inside async context so tokio::spawn works
                let polling_ref = state.polling.clone();
                let db_for_polling = db.clone();
                let registry_for_polling = registry.clone();
                let event_bus_for_polling = event_bus.clone();
                tauri::async_runtime::spawn(async move {
                    polling_ref.start(
                        db_for_polling,
                        registry_for_polling,
                        event_bus_for_polling,
                    );
                });

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
                let engine_for_start = notification_engine.clone();
                let notification_event_rx = event_bus.subscribe();
                tauri::async_runtime::spawn(async move {
                    engine_for_start.reload_rules().await;
                    engine_for_start.start(notification_event_rx);
                });

                // 啟動 AI Scheduler（載入所有已啟用的 AI 規則並啟動定期評估）
                let ai_scheduler_for_start = ai_scheduler.clone();
                tauri::async_runtime::spawn(async move {
                    ai_scheduler_for_start.start().await;
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
