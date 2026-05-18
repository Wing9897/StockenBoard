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
    Ai,
}

impl ConditionType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "price_above" => Some(Self::PriceAbove),
            "price_below" => Some(Self::PriceBelow),
            "change_pct_above" => Some(Self::ChangePctAbove),
            "change_pct_below" => Some(Self::ChangePctBelow),
            "ai" => Some(Self::Ai),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PriceAbove => "price_above",
            Self::PriceBelow => "price_below",
            Self::ChangePctAbove => "change_pct_above",
            Self::ChangePctBelow => "change_pct_below",
            Self::Ai => "ai",
        }
    }
}

// === AI 設定 ===
/// AI 規則的設定結構，包含 prompt、歷史窗口大小、分析間隔
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiConfig {
    pub prompt: String,
    pub history_window: u32,
    pub analysis_interval_secs: u64,
}

impl AiConfig {
    /// 驗證 AiConfig 的所有欄位是否符合規範
    ///
    /// - prompt: 非空字串，最大 2000 字元
    /// - history_window: 1 ≤ n ≤ 100
    /// - analysis_interval_secs: ≥ 30
    pub fn validate(&self) -> Result<(), String> {
        if self.prompt.is_empty() {
            return Err("prompt must not be empty".to_string());
        }
        if self.prompt.chars().count() > 2000 {
            return Err(format!(
                "prompt must not exceed 2000 characters, got {}",
                self.prompt.chars().count()
            ));
        }
        if self.history_window < 1 || self.history_window > 100 {
            return Err(format!(
                "history_window must be between 1 and 100, got {}",
                self.history_window
            ));
        }
        if self.analysis_interval_secs < 30 {
            return Err(format!(
                "analysis_interval_secs must be at least 30, got {}",
                self.analysis_interval_secs
            ));
        }
        Ok(())
    }
}

// === AI Provider 設定 ===
/// AI 服務提供者的連線設定，儲存於 settings 表中
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiProviderConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>, // 已解密的 API key
}

impl AiProviderConfig {
    /// 驗證 AiProviderConfig 的必要欄位
    ///
    /// - base_url: 非空字串
    /// - model: 非空字串
    /// - api_key: 可為 None（適用於本地 Ollama 等無需認證的服務）
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<(), String> {
        if self.base_url.trim().is_empty() {
            return Err("base_url must not be empty".to_string());
        }
        if self.model.trim().is_empty() {
            return Err("model must not be empty".to_string());
        }
        Ok(())
    }
}

// === AI Provider Config Response（安全回應，不含 api_key 原文）===
/// AI 服務提供者設定的回應結構（不包含 api_key 原文，僅回傳是否已設定）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProviderConfigResponse {
    pub base_url: String,
    pub model: String,
    pub has_api_key: bool,
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
#[allow(dead_code)]
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
    pub ai_config: Option<AiConfig>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRuleRequest {
    pub name: Option<String>,
    pub condition_type: Option<String>,
    pub threshold: Option<f64>,
    pub channel_ids: Option<Vec<i64>>,
    pub cooldown_secs: Option<u64>,
    pub ai_config: Option<Option<AiConfig>>,
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

impl NotificationData {
    /// 從 rule_name 中提取 AI reason 文字
    ///
    /// AI 規則的 rule_name 格式為 "[AI] {reason}"，此函數提取 reason 部分。
    /// 若格式不符，則回傳原始 rule_name。
    pub fn ai_reason(&self) -> &str {
        self.rule_name
            .strip_prefix("[AI] ")
            .unwrap_or(&self.rule_name)
    }
}
