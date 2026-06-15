//! CoreState — 共享核心狀態，同時供 Desktop (Tauri) 與 Server (Axum) 使用。
//!
//! 包含資料庫、Provider Registry、Event Bus、通知引擎、AI 排程器、全局冷卻期、輪詢管理器。
//! 不含任何 Tauri 相關依賴。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::broadcast;

#[cfg(feature = "desktop")]
use std::collections::HashMap;
#[cfg(feature = "desktop")]
use tokio::sync::RwLock;
#[cfg(feature = "desktop")]
use tokio::task::JoinHandle;

use crate::db::DbPool;
use crate::events::AppEvent;
use crate::notifications::engine::NotificationEngine;
use crate::notifications::ai_scheduler::AiScheduler;
use crate::notifications::global_cooldown::GlobalCooldown;
use crate::polling::PollingManager;
use crate::providers::registry::ProviderRegistry;
#[cfg(feature = "desktop")]
use crate::providers::WsTickerUpdate;

/// 確保 DB schema 一致 — 版本不同就刪除重建。
///
/// 此函式在 desktop 與 server 模式都會使用，因此放在 core 層。
pub fn ensure_clean_db(app_dir: &Path) {
    let db_path = app_dir.join("stockenboard.db");
    let marker = app_dir.join(".schema_v");
    const SCHEMA_VER: &str = "8"; // Bumped for push notifications tables
    let current = std::fs::read_to_string(&marker).unwrap_or_default();
    if current.trim() != SCHEMA_VER {
        eprintln!(
            "[DB] Schema version mismatch (current={:?}, expected={}), deleting and recreating database",
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

/// 共享核心狀態結構，包含所有與平台無關的元件。
pub struct CoreState {
    /// 統一 DB 存取層
    pub db: Arc<DbPool>,
    /// 共享 Provider Registry（含 rate limiting）
    pub registry: Arc<ProviderRegistry>,
    /// Event Bus（解耦 Polling ↔ DB ↔ 前端）
    pub event_bus: broadcast::Sender<AppEvent>,
    /// 推播通知引擎（規則 CRUD 後需 reload）
    pub notification_engine: Arc<NotificationEngine>,
    /// AI 排程器（管理 AI 規則的定期評估 task）
    pub ai_scheduler: Arc<AiScheduler>,
    /// 全局通知冷卻期（跨規則共享的最小觸發間隔）
    pub global_cooldown: Arc<GlobalCooldown>,
    /// 輪詢管理器
    pub polling: PollingManager,
    /// 資料目錄路徑（icons、theme_bg 等存放位置）
    pub data_dir: PathBuf,

    // ── Desktop-only fields (Tauri WebSocket relay) ──────────────
    /// WebSocket ticker update broadcast sender (desktop only)
    #[cfg(feature = "desktop")]
    pub ws_sender: broadcast::Sender<WsTickerUpdate>,
    /// Active WebSocket task handles keyed by provider ID (desktop only)
    #[cfg(feature = "desktop")]
    #[allow(clippy::type_complexity)]
    pub ws_tasks: RwLock<HashMap<String, (JoinHandle<()>, JoinHandle<()>)>>,
}

impl CoreState {
    /// 初始化所有共享元件。
    ///
    /// 1. 確保 DB schema 一致（`ensure_clean_db`）
    /// 2. 開啟資料庫
    /// 3. 建立 Provider Registry
    /// 4. 建立 Event Bus
    /// 5. 從 DB 讀取 global cooldown 設定
    /// 6. 建立 NotificationEngine
    /// 7. 建立 AiScheduler
    /// 8. 建立 PollingManager
    pub fn new(data_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        ensure_clean_db(data_dir);

        let db_path = data_dir.join("stockenboard.db");
        let db = Arc::new(DbPool::open(&db_path)?);

        let registry = Arc::new(ProviderRegistry::new());

        let (event_bus, _) = broadcast::channel::<AppEvent>(512);

        let cooldown_secs: u64 = db
            .get_setting("notification_global_cooldown")
            .ok()
            .flatten()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        let global_cooldown = Arc::new(GlobalCooldown::new(cooldown_secs));

        let notification_engine = Arc::new(NotificationEngine::new(
            db.clone(),
            event_bus.clone(),
            global_cooldown.clone(),
        ));

        let ai_scheduler = Arc::new(
            AiScheduler::new(db.clone())
                .with_event_bus(event_bus.clone())
                .with_global_cooldown(global_cooldown.clone()),
        );

        let polling = PollingManager::new();

        Ok(Self {
            db,
            registry,
            event_bus,
            notification_engine,
            ai_scheduler,
            global_cooldown,
            polling,
            data_dir: data_dir.to_path_buf(),
            #[cfg(feature = "desktop")]
            ws_sender: broadcast::channel(256).0,
            #[cfg(feature = "desktop")]
            ws_tasks: RwLock::new(HashMap::new()),
        })
    }

    /// 啟動背景任務：Notification Engine 與 AI Scheduler。
    ///
    /// 注意：Polling 不在此啟動，因為在 desktop 模式下需要 `tauri::AppHandle`，
    /// 而在 server 模式下將以不同方式啟動。Polling 啟動由呼叫方自行處理。
    pub async fn start_background_tasks(&self) {
        // 啟動 Notification Engine：先載入規則，再訂閱 event bus
        let engine = self.notification_engine.clone();
        let notification_event_rx = self.event_bus.subscribe();
        tokio::spawn(async move {
            engine.reload_rules().await;
            engine.start(notification_event_rx);
        });

        // 啟動 AI Scheduler（載入所有已啟用的 AI 規則並啟動定期評估）
        let scheduler = self.ai_scheduler.clone();
        tokio::spawn(async move {
            scheduler.start().await;
        });
    }
}
