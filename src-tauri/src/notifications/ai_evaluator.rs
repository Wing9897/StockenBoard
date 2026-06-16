//! AI 評估器模組
//!
//! 負責組裝 prompt、呼叫 AI API、解析 AI 回應。
//! 此模組為 AI 通知規則的核心評估邏輯。

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use crate::db::DbPool;
use crate::notifications::models::{AiConfig, AiProviderConfig};

// === 錯誤類型 ===

/// AI 評估過程中可能發生的錯誤
#[derive(Debug)]
pub enum AiEvalError {
    /// AI 回應不是合法的 JSON 格式
    InvalidJson(String),
    /// JSON 中缺少必要欄位或欄位型別不正確
    MissingField(String),
    /// AI API 請求失敗（網路錯誤、逾時等）
    RequestFailed(String),
    /// 資料庫操作失敗
    DatabaseError(String),
    /// 無可用的價格歷史紀錄
    NoPriceHistory,
}

impl fmt::Display for AiEvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(raw) => write!(f, "AI response is not valid JSON: {}", raw),
            Self::MissingField(detail) => {
                write!(f, "AI response missing required field: {}", detail)
            }
            Self::RequestFailed(reason) => write!(f, "AI API request failed: {}", reason),
            Self::DatabaseError(reason) => write!(f, "Database error: {}", reason),
            Self::NoPriceHistory => write!(f, "No price history records available"),
        }
    }
}

impl std::error::Error for AiEvalError {}

// === 資料結構 ===

/// AI 模型回傳的回應結構
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiResponse {
    /// 是否觸發通知
    pub trigger: bool,
    /// AI 的判斷理由說明
    pub reason: String,
}

/// 用於組裝 AI prompt 的聊天訊息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    /// 訊息角色（"system" 或 "user"）
    pub role: String,
    /// 訊息內容
    pub content: String,
}

// === 公開函數 ===

/// 從 AI 回應文字中解析 JSON，支援 markdown code block 提取
///
/// # Arguments
/// * `raw` - AI 模型回傳的原始文字
///
/// # Returns
/// * `Ok(AiResponse)` - 成功解析的回應
/// * `Err(AiEvalError)` - 解析失敗（非 JSON、缺少欄位等）
pub fn parse_ai_response(raw: &str) -> Result<AiResponse, AiEvalError> {
    let trimmed = raw.trim();

    // Step 1: Try to parse directly as JSON
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return validate_ai_response_json(&value, raw);
    }

    // Step 2: Try to extract JSON from markdown code blocks
    if let Some(extracted) = extract_json_from_markdown(trimmed) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&extracted) {
            return validate_ai_response_json(&value, raw);
        }
    }

    // Neither direct parse nor markdown extraction worked
    eprintln!("[AiEvaluator] AI response is not valid JSON, raw: {}", raw);
    Err(AiEvalError::InvalidJson(raw.to_string()))
}

/// 從 markdown code block 中提取 JSON 內容
///
/// 支援格式：
/// - ```json\n...\n```
/// - ```\n...\n```
fn extract_json_from_markdown(text: &str) -> Option<String> {
    // Try ```json\n...\n``` first
    if let Some(start) = text.find("```json") {
        let content_start = start + "```json".len();
        let rest = &text[content_start..];
        if let Some(end) = rest.find("```") {
            let extracted = rest[..end].trim().to_string();
            if !extracted.is_empty() {
                return Some(extracted);
            }
        }
    }

    // Try ```\n...\n```
    if let Some(start) = text.find("```") {
        let content_start = start + "```".len();
        let rest = &text[content_start..];
        // Skip to next line if there's a newline right after ```
        let content = rest.strip_prefix('\n').unwrap_or(rest);
        if let Some(end) = content.find("```") {
            let extracted = content[..end].trim().to_string();
            if !extracted.is_empty() {
                return Some(extracted);
            }
        }
    }

    None
}

/// 驗證解析後的 JSON 值包含必要欄位且型別正確
fn validate_ai_response_json(
    value: &serde_json::Value,
    raw: &str,
) -> Result<AiResponse, AiEvalError> {
    let obj = match value.as_object() {
        Some(obj) => obj,
        None => {
            eprintln!("[AiEvaluator] AI response is not a JSON object, raw: {}", raw);
            return Err(AiEvalError::MissingField(
                "response is not a JSON object".to_string(),
            ));
        }
    };

    // Validate "trigger" field exists and is boolean
    let trigger = match obj.get("trigger") {
        Some(v) => match v.as_bool() {
            Some(b) => b,
            None => {
                eprintln!("[AiEvaluator] 'trigger' field is not boolean, raw: {}", raw);
                return Err(AiEvalError::MissingField(
                    "\"trigger\" field is not a boolean".to_string(),
                ));
            }
        },
        None => {
            eprintln!("[AiEvaluator] Missing 'trigger' field, raw: {}", raw);
            return Err(AiEvalError::MissingField(
                "missing \"trigger\" field".to_string(),
            ));
        }
    };

    // Validate "reason" field exists and is string
    let reason = match obj.get("reason") {
        Some(v) => match v.as_str() {
            Some(s) => s.to_string(),
            None => {
                eprintln!("[AiEvaluator] 'reason' field is not a string, raw: {}", raw);
                return Err(AiEvalError::MissingField(
                    "\"reason\" field is not a string".to_string(),
                ));
            }
        },
        None => {
            eprintln!("[AiEvaluator] Missing 'reason' field, raw: {}", raw);
            return Err(AiEvalError::MissingField(
                "missing \"reason\" field".to_string(),
            ));
        }
    };

    Ok(AiResponse { trigger, reason })
}

/// 組裝送交 AI 的 prompt（system message + user message）
///
/// # Arguments
/// * `user_condition` - 使用者設定的自然語言觸發條件
/// * `price_history` - 最近的價格歷史紀錄
///
/// # Returns
/// 組裝好的 ChatMessage 向量，包含 system 和 user 訊息
pub fn build_prompt(user_condition: &str, price_history: &[PriceRecord]) -> Vec<ChatMessage> {
    let system_content = concat!(
        "你是一個金融市場分析助手。根據提供的價格歷史數據和使用者的觸發條件，判斷是否應該觸發通知。\n\n",
        "分析指引：\n",
        "- 紀錄按時間由新到舊排列\n",
        "- 「漲跌幅(%)」是每筆相對前一筆的變化率\n",
        "- 判斷整體趨勢時，計算「最新價格」相對「最舊價格」的總變化百分比：(最新-最舊)/最舊×100\n",
        "- 判斷成交量異常時，比較最近幾筆的成交量與較早幾筆的平均成交量\n\n",
        "你必須以 JSON 格式回應：{\"trigger\": true/false, \"reason\": \"簡短說明理由\"}\n\n",
        "只回傳 JSON，不要包含其他文字。"
    ).to_string();

    let formatted_rows: String = price_history
        .iter()
        .map(|record| {
            format!(
                "| {} | {:.2} | {:+.1}% | {:.1} |",
                record.recorded_at, record.price, record.change_pct, record.volume,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Pre-calculate summary stats to reduce model's computation burden
    let summary = if price_history.len() >= 2 {
        let newest = &price_history[0];
        let oldest = &price_history[price_history.len() - 1];
        let total_change_pct = (newest.price - oldest.price) / oldest.price * 100.0;
        let avg_volume: f64 = price_history.iter().map(|r| r.volume).sum::<f64>()
            / price_history.len() as f64;
        let max_volume = price_history
            .iter()
            .map(|r| r.volume)
            .fold(f64::NEG_INFINITY, f64::max);
        format!(
            "\n\n摘要統計：\n- 整體價格變化：{:.2} → {:.2}（{:+.2}%）\n- 平均成交量：{:.1}\n- 最大成交量：{:.1}（為平均的 {:.1} 倍）",
            oldest.price, newest.price, total_change_pct,
            avg_volume, max_volume, max_volume / avg_volume
        )
    } else {
        String::new()
    };

    let user_content = format!(
        "觸發條件：{}\n\n最近 {} 筆價格紀錄（由新到舊）：\n| 時間 | 價格 | 漲跌幅(%) | 成交量 |\n|------|------|-----------|--------|\n{}{}\n\n請判斷是否滿足觸發條件。",
        user_condition,
        price_history.len(),
        formatted_rows,
        summary,
    );

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_content,
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_content,
        },
    ]
}

/// 價格歷史紀錄，用於組裝 prompt
#[derive(Debug, Clone)]
pub struct PriceRecord {
    pub price: f64,
    pub change_pct: f64,
    pub volume: f64,
    pub recorded_at: String,
}

// === evaluate_ai_rule ===

/// 執行一次 AI 評估：取得歷史資料、建構 prompt、呼叫 API、解析回應
///
/// # Arguments
/// * `db` - 資料庫連線池
/// * `http_client` - HTTP 客戶端（reqwest）
/// * `rule_id` - 通知規則 ID（用於日誌）
/// * `subscription_id` - 目標訂閱 ID
/// * `ai_config` - AI 規則設定（含 prompt、history_window）
/// * `provider_config` - AI 服務提供者設定（含 base_url、model、api_key）
///
/// # Returns
/// * `Ok(AiResponse)` - AI 判斷結果
/// * `Err(AiEvalError)` - 評估過程中的錯誤
pub async fn evaluate_ai_rule(
    db: &Arc<DbPool>,
    http_client: &reqwest::Client,
    rule_id: i64,
    subscription_id: i64,
    ai_config: &AiConfig,
    provider_config: &AiProviderConfig,
) -> Result<AiResponse, AiEvalError> {
    // Step 1: Fetch recent price history records
    let history_rows = db
        .get_price_history(subscription_id, None, None, ai_config.history_window as i64)
        .map_err(|e| {
            eprintln!("[AiEvaluator] rule_id={} failed to get price history: {}", rule_id, e);
            AiEvalError::DatabaseError(e)
        })?;

    if history_rows.is_empty() {
        eprintln!(
            "[AiEvaluator] rule_id={} subscription_id={} no price history available",
            rule_id, subscription_id
        );
        return Err(AiEvalError::NoPriceHistory);
    }

    // Step 2: Convert DB rows to PriceRecord structs
    let price_records: Vec<PriceRecord> = history_rows
        .iter()
        .map(|row| PriceRecord {
            price: row.price,
            change_pct: row.change_pct.unwrap_or(0.0),
            volume: row.volume.unwrap_or(0.0),
            recorded_at: format_timestamp(row.recorded_at),
        })
        .collect();

    // Step 3: Build prompt
    let messages = build_prompt(&ai_config.prompt, &price_records);

    // Step 4: Make HTTP POST request to AI API
    let url = format!(
        "{}/chat/completions",
        provider_config.base_url.trim_end_matches('/')
    );

    let mut request_body = serde_json::json!({
        "model": provider_config.model,
        "messages": messages,
        "temperature": 0.1,
        "max_tokens": 2000,
        "response_format": {"type": "json_object"}
    });
    // Disable thinking if configured (helps thinking models produce clean JSON)
    if provider_config.disable_thinking {
        request_body["reasoning_effort"] = serde_json::json!("none");
    }

    let mut request = http_client
        .post(&url)
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(30));

    // Add Authorization header only if api_key is present
    if let Some(ref api_key) = provider_config.api_key {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = request.json(&request_body).send().await.map_err(|e| {
        eprintln!("[AiEvaluator] rule_id={} AI API request failed: {}", rule_id, e);
        AiEvalError::RequestFailed(e.to_string())
    })?;

    // Check HTTP status
    let status = response.status();
    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        eprintln!(
            "[AiEvaluator] rule_id={} AI API returned HTTP {}: {}",
            rule_id, status, error_body
        );
        return Err(AiEvalError::RequestFailed(format!(
            "HTTP {} - {}",
            status, error_body
        )));
    }

    // Step 5: Parse response body to extract AI's message content
    let response_json: serde_json::Value = response.json().await.map_err(|e| {
        eprintln!(
            "[AiEvaluator] rule_id={} failed to parse AI API response JSON: {}",
            rule_id, e
        );
        AiEvalError::RequestFailed(format!("Failed to parse response body: {}", e))
    })?;

    // Extract the message content from OpenAI-compatible response format
    // Expected: { "choices": [{ "message": { "content": "..." } }] }
    // Fallback: Some thinking models (e.g. qwen3.5) put the answer in "reasoning" field
    // when content is empty, so we check both.
    let message = response_json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"));

    let content = message
        .and_then(|m| {
            // Try "content" first
            let c = m.get("content").and_then(|v| v.as_str()).unwrap_or("");
            if !c.is_empty() {
                return Some(c);
            }
            // Fallback: try "reasoning" field (thinking models like qwen3.5)
            m.get("reasoning").and_then(|v| v.as_str())
        })
        .ok_or_else(|| {
            eprintln!(
                "[AiEvaluator] rule_id={} AI API response format invalid, cannot extract content: {}",
                rule_id, response_json
            );
            AiEvalError::RequestFailed(
                "AI API response missing choices[0].message.content".to_string(),
            )
        })?;

    // Step 6: Parse AI response content
    let ai_response = parse_ai_response(content).map_err(|e| {
        eprintln!(
            "[AiEvaluator] rule_id={} failed to parse AI response content: {}",
            rule_id, e
        );
        e
    })?;

    Ok(ai_response)
}

/// 將 Unix timestamp 格式化為人類可讀的日期時間字串
fn format_timestamp(ts: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| ts.to_string())
}
