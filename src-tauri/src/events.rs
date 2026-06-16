/// AppEvent — 統一的應用程式事件類型
/// 用於 Event Bus 解耦 Polling、DB 寫入、前端通知
use crate::icons::DownloadProgress;
use crate::providers::AssetData;
use serde::Serialize;

#[derive(Clone, Debug)]
pub enum AppEvent {
    /// 價格更新（從 Polling/WS 取得）
    PriceUpdate {
        provider_id: String,
        data: Vec<AssetData>,
        /// 需要寫入歷史的 symbol 集合
        record_symbols: Vec<String>,
    },
    /// 價格錯誤
    PriceError {
        provider_id: String,
        symbols: Vec<String>,
        error: String,
    },
    /// Polling tick（每次 fetch 完成後發送）
    PollTick {
        provider_id: String,
        fetched_at: i64,
        interval_ms: u64,
    },
    /// 通知規則觸發（閾值或 AI）— 供前端側欄即時顯示
    NotificationTriggered(NotificationTriggeredPayload),
    /// System notification request — dispatched when a rule with a system channel fires
    SystemNotification {
        title: String,
        body: String,
    },
    /// Logo download progress — forwarded from download_all_logos broadcast channel
    LogoDownloadProgress(DownloadProgress),
}

/// 前端事件用的通知觸發 payload（規則觸發即時推送到 UI）
#[derive(Debug, Clone, Serialize)]
pub struct NotificationTriggeredPayload {
    pub rule_name: String,
    pub symbol: String,
    pub provider: String,
    pub price: f64,
    pub condition_type: String,
    pub threshold: f64,
    /// Unix 秒
    pub triggered_at: i64,
    pub is_ai: bool,
    /// AI 規則的判斷理由；非 AI 規則為 None
    pub ai_reason: Option<String>,
}

/// 前端事件用的 PollTick 結構
#[derive(Debug, Clone, Serialize)]
pub struct PollTickPayload {
    pub provider_id: String,
    pub fetched_at: i64,
    pub interval_ms: u64,
}
