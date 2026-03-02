/// AppEvent — 統一的應用程式事件類型
/// 用於 Event Bus 解耦 Polling、DB 寫入、前端通知
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
}

/// 前端事件用的 PollTick 結構
#[derive(Debug, Clone, Serialize)]
pub struct PollTickPayload {
    pub provider_id: String,
    pub fetched_at: i64,
    pub interval_ms: u64,
}
