//! 推播通知資料模型定義
//!
//! 包含 ConditionType、ChannelType、NotificationRule、TelegramConfig、WebhookConfig 等結構。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// === 條件類型 ===
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionType {
    PriceAbove,
    PriceBelow,
    ChangePctAbove,
    ChangePctBelow,
}

impl ConditionType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "price_above" => Some(Self::PriceAbove),
            "price_below" => Some(Self::PriceBelow),
            "change_pct_above" => Some(Self::ChangePctAbove),
            "change_pct_below" => Some(Self::ChangePctBelow),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PriceAbove => "price_above",
            Self::PriceBelow => "price_below",
            Self::ChangePctAbove => "change_pct_above",
            Self::ChangePctBelow => "change_pct_below",
        }
    }
}

// === 通道類型 ===
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Telegram,
    Webhook,
}

impl ChannelType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "telegram" => Some(Self::Telegram),
            "webhook" => Some(Self::Webhook),
            _ => None,
        }
    }
}

// === 通道設定 ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
}

// === 通知規則（記憶體中的完整結構）===
#[derive(Debug, Clone)]
pub struct NotificationRule {
    pub id: i64,
    pub name: String,
    pub subscription_id: i64,
    pub provider_id: String,
    pub symbol: String,
    pub condition_type: ConditionType,
    pub threshold: f64,
    pub channel_ids: Vec<i64>,
    pub cooldown_secs: u64,
    pub enabled: bool,
}

// === API 請求結構 ===
#[derive(Debug, Deserialize)]
pub struct CreateRuleRequest {
    pub name: String,
    pub subscription_id: i64,
    pub condition_type: String,
    pub threshold: f64,
    pub channel_ids: Vec<i64>,
    pub cooldown_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRuleRequest {
    pub name: Option<String>,
    pub condition_type: Option<String>,
    pub threshold: Option<f64>,
    pub channel_ids: Option<Vec<i64>>,
    pub cooldown_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SaveChannelRequest {
    pub channel_type: String,
    pub name: String,
    pub config: String, // JSON string of TelegramConfig or WebhookConfig
}

// === 通知資料（用於格式化訊息）===
#[derive(Debug, Clone)]
pub struct NotificationData {
    pub symbol: String,
    pub provider: String,
    pub price: f64,
    pub condition_type: ConditionType,
    pub threshold: f64,
    pub rule_name: String,
    pub triggered_at: chrono::DateTime<chrono::Utc>,
}
