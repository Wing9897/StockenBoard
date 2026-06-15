use serde::{Deserialize, Serialize};

// ── Shared complex-type aliases ─────────────────────────────────

/// 一筆待寫入的價格紀錄：(symbol, price, change_pct, volume, pre_price, post_price)
pub type PriceRecord = (
    String,
    f64,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
);

/// Polling 用的 provider 設定值：(api_key, api_secret, api_url, refresh_interval)
pub type PollingProviderSetting = (Option<String>, Option<String>, Option<String>, Option<i64>);

// ── Data types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: i64,
    pub sub_type: String,
    pub symbol: String,
    pub display_name: Option<String>,
    pub selected_provider_id: String,
    pub asset_type: String,
    pub pool_address: Option<String>,
    pub token_from_address: Option<String>,
    pub token_to_address: Option<String>,
    pub sort_order: i64,
    pub record_enabled: i64,
    pub record_from_hour: Option<i64>,
    pub record_to_hour: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettingsRow {
    pub provider_id: String,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub api_url: Option<String>,
    pub refresh_interval: Option<i64>,
    pub connection_type: String,
    pub record_from_hour: Option<i64>,
    pub record_to_hour: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewRow {
    pub id: i64,
    pub name: String,
    pub view_type: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewSubCount {
    pub view_id: i64,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceHistoryRow {
    pub id: i64,
    pub subscription_id: i64,
    pub provider_id: String,
    pub price: f64,
    pub change_pct: Option<f64>,
    pub volume: Option<f64>,
    pub pre_price: Option<f64>,
    pub post_price: Option<f64>,
    pub recorded_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryStats {
    pub total: i64,
    pub oldest: Option<i64>,
    pub newest: Option<i64>,
}

// ── Notification types ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannelRow {
    pub id: i64,
    pub channel_type: String,
    pub name: String,
    pub config: String, // JSON string
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRuleRow {
    pub id: i64,
    pub name: String,
    pub subscription_id: i64,
    pub condition_type: String,
    pub threshold: f64,
    pub channel_ids: String, // JSON array string
    pub cooldown_secs: i64,
    pub enabled: bool,
    pub ai_config: Option<String>, // JSON string for AI rules
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationHistoryRow {
    pub id: i64,
    pub rule_id: i64,
    pub channel_id: i64,
    pub status: String,
    pub price: f64,
    pub message: String,
    pub error: Option<String>,
    pub sent_at: i64,
}

// ── Export/Import types ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub subscriptions: Vec<ExportSubscription>,
    pub views: Vec<ExportView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSubscription {
    pub symbol: String,
    pub display_name: Option<String>,
    pub selected_provider_id: String,
    pub asset_type: String,
    pub sub_type: String,
    pub pool_address: Option<String>,
    pub token_from_address: Option<String>,
    pub token_to_address: Option<String>,
    pub record_enabled: Option<bool>,
    pub record_from_hour: Option<i64>,
    pub record_to_hour: Option<i64>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportView {
    pub name: String,
    pub view_type: String,
    pub symbols: Vec<String>,
}
