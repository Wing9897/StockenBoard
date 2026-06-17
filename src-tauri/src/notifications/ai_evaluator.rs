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
use crate::notifications::token_estimator::compute_trimmed_window;

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

/// 組裝多訂閱的 AI prompt（system message + user message）
///
/// 每個訂閱會產生一個獨立的標記區段，包含符號名稱、紀錄數量與格式化的價格表格。
/// AI 模型根據所有訂閱的數據判斷是否觸發條件。
///
/// # Arguments
/// * `user_condition` - 使用者設定的自然語言觸發條件
/// * `subscriptions_data` - 多個訂閱的資料，每項包含 (符號名稱, 價格紀錄)
///
/// # Returns
/// 組裝好的 ChatMessage 向量，包含 system 和 user 訊息
pub fn build_multi_prompt(
    user_condition: &str,
    subscriptions_data: &[(String, Vec<PriceRecord>)],
) -> Vec<ChatMessage> {
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

    let mut sections = Vec::new();
    for (symbol, records) in subscriptions_data {
        let formatted_rows: String = records
            .iter()
            .map(|r| {
                format!(
                    "| {} | {:.2} | {:+.1}% | {:.1} |",
                    r.recorded_at, r.price, r.change_pct, r.volume
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        sections.push(format!(
            "## {}\n最近 {} 筆價格紀錄（由新到舊）：\n| 時間 | 價格 | 漲跌幅(%) | 成交量 |\n|------|------|-----------|--------|\n{}",
            symbol,
            records.len(),
            formatted_rows
        ));
    }

    let user_content = format!(
        "觸發條件：{}\n\n{}\n\n請根據以上多資產數據判斷是否滿足觸發條件。",
        user_condition,
        sections.join("\n\n")
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

// === evaluate_ai_rule (test-only) ===

/// 單一訂閱版 AI 評估 — 僅供整合測試使用。
/// 生產代碼已全面改用 `evaluate_ai_rule_multi`。
#[cfg(test)]
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

/// 執行一次多訂閱 AI 評估：解析訂閱 ID、自動裁剪歷史窗口、取得多筆歷史資料、
/// 組裝多訂閱 prompt、呼叫 AI API、解析回應。
///
/// # Arguments
/// * `db` - 資料庫連線池
/// * `http_client` - HTTP 客戶端（reqwest）
/// * `rule_id` - 通知規則 ID（用於日誌）
/// * `subscription_ids` - 目標訂閱 ID 列表
/// * `ai_config` - AI 規則設定（含 prompt、history_window）
/// * `provider_config` - AI 服務提供者設定（含 base_url、model、api_key）
/// * `max_context_tokens` - 最大上下文 token 數（用於自動裁剪）
///
/// # Returns
/// * `Ok(AiResponse)` - AI 判斷結果
/// * `Err(AiEvalError)` - 評估過程中的錯誤
pub async fn evaluate_ai_rule_multi(
    db: &Arc<DbPool>,
    http_client: &reqwest::Client,
    rule_id: i64,
    subscription_ids: &[i64],
    ai_config: &AiConfig,
    provider_config: &AiProviderConfig,
    max_context_tokens: Option<u32>,
) -> Result<AiResponse, AiEvalError> {
    // Step 1: Compute the trimmed history window
    let trimmed_window = compute_trimmed_window(
        subscription_ids.len() as u32,
        ai_config.history_window,
        max_context_tokens,
    );

    // Step 2: Fetch price history for each subscription and resolve symbols
    let all_subscriptions = db.list_all_subscriptions().map_err(|e| {
        eprintln!("[AiEvaluator] rule_id={} failed to list subscriptions: {}", rule_id, e);
        AiEvalError::DatabaseError(e)
    })?;

    let mut subscriptions_data: Vec<(String, Vec<PriceRecord>)> = Vec::new();

    for &sub_id in subscription_ids {
        // Find the subscription symbol
        let symbol = match all_subscriptions.iter().find(|s| s.id == sub_id) {
            Some(s) => s.symbol.clone(),
            None => {
                eprintln!(
                    "[AiEvaluator] rule_id={} subscription_id={} not found, skipping",
                    rule_id, sub_id
                );
                continue;
            }
        };

        // Fetch price history with trimmed window
        let history_rows = db
            .get_price_history(sub_id, None, None, trimmed_window as i64)
            .map_err(|e| {
                eprintln!(
                    "[AiEvaluator] rule_id={} sub_id={} failed to get price history: {}",
                    rule_id, sub_id, e
                );
                AiEvalError::DatabaseError(e)
            })?;

        if history_rows.is_empty() {
            eprintln!(
                "[AiEvaluator] rule_id={} sub_id={} no price history, skipping",
                rule_id, sub_id
            );
            continue;
        }

        let price_records: Vec<PriceRecord> = history_rows
            .iter()
            .map(|row| PriceRecord {
                price: row.price,
                change_pct: row.change_pct.unwrap_or(0.0),
                volume: row.volume.unwrap_or(0.0),
                recorded_at: format_timestamp(row.recorded_at),
            })
            .collect();

        subscriptions_data.push((symbol, price_records));
    }

    // Step 3: If all subscriptions are missing, return error
    if subscriptions_data.is_empty() {
        eprintln!(
            "[AiEvaluator] rule_id={} all subscriptions missing or have no price history",
            rule_id
        );
        return Err(AiEvalError::NoPriceHistory);
    }

    // Step 4: Build multi-subscription prompt
    let messages = build_multi_prompt(&ai_config.prompt, &subscriptions_data);

    // Step 5: Make HTTP POST request to AI API
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
    if provider_config.disable_thinking {
        request_body["reasoning_effort"] = serde_json::json!("none");
    }

    let mut request = http_client
        .post(&url)
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(30));

    if let Some(ref api_key) = provider_config.api_key {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = request.json(&request_body).send().await.map_err(|e| {
        eprintln!("[AiEvaluator] rule_id={} AI API request failed: {}", rule_id, e);
        AiEvalError::RequestFailed(e.to_string())
    })?;

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

    // Step 6: Parse response body
    let response_json: serde_json::Value = response.json().await.map_err(|e| {
        eprintln!(
            "[AiEvaluator] rule_id={} failed to parse AI API response JSON: {}",
            rule_id, e
        );
        AiEvalError::RequestFailed(format!("Failed to parse response body: {}", e))
    })?;

    let message = response_json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"));

    let content = message
        .and_then(|m| {
            let c = m.get("content").and_then(|v| v.as_str()).unwrap_or("");
            if !c.is_empty() {
                return Some(c);
            }
            m.get("reasoning").and_then(|v| v.as_str())
        })
        .ok_or_else(|| {
            eprintln!(
                "[AiEvaluator] rule_id={} AI API response format invalid: {}",
                rule_id, response_json
            );
            AiEvalError::RequestFailed(
                "AI API response missing choices[0].message.content".to_string(),
            )
        })?;

    // Step 7: Parse AI response content
    let ai_response = parse_ai_response(content).map_err(|e| {
        eprintln!(
            "[AiEvaluator] rule_id={} failed to parse AI response content: {}",
            rule_id, e
        );
        e
    })?;

    Ok(ai_response)
}

/// 解析 `subscription_ids` JSON 字串為 `Vec<i64>`，
/// 當欄位為 NULL 或包含無效 JSON 時回退使用 `[fallback_subscription_id]`。
///
/// # Arguments
/// * `subscription_ids_json` - 來自資料庫的 JSON 陣列字串（可為 None）
/// * `fallback_subscription_id` - 回退用的單一訂閱 ID（用於舊規則向後相容）
///
/// # Returns
/// 解析後的訂閱 ID 列表，保證至少包含一個元素
pub fn resolve_subscription_ids(
    subscription_ids_json: &Option<String>,
    fallback_subscription_id: i64,
) -> Vec<i64> {
    match subscription_ids_json {
        None => vec![fallback_subscription_id],
        Some(json_str) => {
            match serde_json::from_str::<Vec<i64>>(json_str) {
                Ok(ids) if !ids.is_empty() => ids,
                _ => vec![fallback_subscription_id],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === build_multi_prompt tests ===

    #[test]
    fn test_build_multi_prompt_single_subscription() {
        let data = vec![(
            "BTC".to_string(),
            vec![
                PriceRecord {
                    price: 100.0,
                    change_pct: 1.5,
                    volume: 500.0,
                    recorded_at: "2024-01-01 12:00".to_string(),
                },
            ],
        )];

        let messages = build_multi_prompt("BTC 跌幅超過 5%", &data);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");

        // User message should contain the condition
        assert!(messages[1].content.contains("觸發條件：BTC 跌幅超過 5%"));
        // Should contain the symbol section header
        assert!(messages[1].content.contains("## BTC"));
        // Should contain record count
        assert!(messages[1].content.contains("最近 1 筆價格紀錄"));
        // Should contain formatted row
        assert!(messages[1].content.contains("| 2024-01-01 12:00 | 100.00 | +1.5% | 500.0 |"));
        // Should contain multi-asset closing instruction
        assert!(messages[1].content.contains("請根據以上多資產數據判斷是否滿足觸發條件。"));
    }

    #[test]
    fn test_build_multi_prompt_multiple_subscriptions() {
        let data = vec![
            (
                "BTC".to_string(),
                vec![
                    PriceRecord {
                        price: 50000.0,
                        change_pct: -2.3,
                        volume: 1000.0,
                        recorded_at: "2024-01-02 10:00".to_string(),
                    },
                    PriceRecord {
                        price: 51000.0,
                        change_pct: 0.5,
                        volume: 800.0,
                        recorded_at: "2024-01-01 10:00".to_string(),
                    },
                ],
            ),
            (
                "ETH".to_string(),
                vec![
                    PriceRecord {
                        price: 3000.0,
                        change_pct: -1.0,
                        volume: 2000.0,
                        recorded_at: "2024-01-02 10:00".to_string(),
                    },
                ],
            ),
        ];

        let messages = build_multi_prompt("BTC 與 ETH 同時下跌", &data);

        assert_eq!(messages.len(), 2);
        // Should contain both symbols
        assert!(messages[1].content.contains("## BTC"));
        assert!(messages[1].content.contains("## ETH"));
        // BTC section should say 2 records
        assert!(messages[1].content.contains("最近 2 筆價格紀錄"));
        // ETH section should say 1 record
        // Check for both sections with their respective record counts
        let btc_idx = messages[1].content.find("## BTC").unwrap();
        let eth_idx = messages[1].content.find("## ETH").unwrap();
        // BTC appears before ETH
        assert!(btc_idx < eth_idx);
    }

    #[test]
    fn test_build_multi_prompt_empty_subscriptions() {
        let data: Vec<(String, Vec<PriceRecord>)> = vec![];
        let messages = build_multi_prompt("test condition", &data);

        assert_eq!(messages.len(), 2);
        assert!(messages[1].content.contains("觸發條件：test condition"));
        // No sections but closing instruction still present
        assert!(messages[1].content.contains("請根據以上多資產數據判斷是否滿足觸發條件。"));
    }

    #[test]
    fn test_build_multi_prompt_system_message_matches_single() {
        let single_messages = build_prompt("test", &[PriceRecord {
            price: 100.0,
            change_pct: 0.0,
            volume: 100.0,
            recorded_at: "2024-01-01 00:00".to_string(),
        }]);

        let multi_messages = build_multi_prompt("test", &[(
            "BTC".to_string(),
            vec![PriceRecord {
                price: 100.0,
                change_pct: 0.0,
                volume: 100.0,
                recorded_at: "2024-01-01 00:00".to_string(),
            }],
        )]);

        // System message should be identical
        assert_eq!(single_messages[0].content, multi_messages[0].content);
    }

    // === resolve_subscription_ids tests ===

    #[test]
    fn test_resolve_subscription_ids_none_returns_fallback() {
        let result = resolve_subscription_ids(&None, 42);
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn test_resolve_subscription_ids_valid_json_array() {
        let json = Some("[1, 3, 7]".to_string());
        let result = resolve_subscription_ids(&json, 99);
        assert_eq!(result, vec![1, 3, 7]);
    }

    #[test]
    fn test_resolve_subscription_ids_invalid_json_returns_fallback() {
        let json = Some("not valid json".to_string());
        let result = resolve_subscription_ids(&json, 5);
        assert_eq!(result, vec![5]);
    }

    #[test]
    fn test_resolve_subscription_ids_empty_array_returns_fallback() {
        let json = Some("[]".to_string());
        let result = resolve_subscription_ids(&json, 10);
        assert_eq!(result, vec![10]);
    }

    #[test]
    fn test_resolve_subscription_ids_single_element_array() {
        let json = Some("[42]".to_string());
        let result = resolve_subscription_ids(&json, 99);
        assert_eq!(result, vec![42]);
    }

    // === Property 2: Multi-subscription prompt completeness ===
    // Validates: Requirements 1.5
    // Verifies that build_multi_prompt output contains all symbols and all price records from input data.

    #[test]
    fn test_multi_prompt_completeness_contains_all_symbols() {
        let data = vec![
            (
                "BTC".to_string(),
                vec![PriceRecord {
                    price: 67000.0,
                    change_pct: 2.1,
                    volume: 1200.0,
                    recorded_at: "2024-03-01 08:00".to_string(),
                }],
            ),
            (
                "ETH".to_string(),
                vec![PriceRecord {
                    price: 3500.0,
                    change_pct: -0.5,
                    volume: 800.0,
                    recorded_at: "2024-03-01 08:00".to_string(),
                }],
            ),
            (
                "SOL".to_string(),
                vec![PriceRecord {
                    price: 145.0,
                    change_pct: 5.3,
                    volume: 3000.0,
                    recorded_at: "2024-03-01 08:00".to_string(),
                }],
            ),
        ];

        let messages = build_multi_prompt("cross-asset correlation check", &data);
        let user_msg = &messages[1].content;

        // All symbol names must appear as section headers
        assert!(user_msg.contains("## BTC"), "user message should contain BTC symbol");
        assert!(user_msg.contains("## ETH"), "user message should contain ETH symbol");
        assert!(user_msg.contains("## SOL"), "user message should contain SOL symbol");
    }

    #[test]
    fn test_multi_prompt_completeness_contains_all_price_records() {
        let data = vec![
            (
                "BTC".to_string(),
                vec![
                    PriceRecord {
                        price: 67000.50,
                        change_pct: 2.1,
                        volume: 1200.5,
                        recorded_at: "2024-03-02 12:00".to_string(),
                    },
                    PriceRecord {
                        price: 65500.75,
                        change_pct: -1.8,
                        volume: 900.3,
                        recorded_at: "2024-03-01 12:00".to_string(),
                    },
                ],
            ),
            (
                "ETH".to_string(),
                vec![
                    PriceRecord {
                        price: 3500.25,
                        change_pct: 0.9,
                        volume: 2100.0,
                        recorded_at: "2024-03-02 12:00".to_string(),
                    },
                    PriceRecord {
                        price: 3450.00,
                        change_pct: -0.3,
                        volume: 1800.7,
                        recorded_at: "2024-03-01 12:00".to_string(),
                    },
                    PriceRecord {
                        price: 3460.10,
                        change_pct: 1.2,
                        volume: 1500.0,
                        recorded_at: "2024-02-29 12:00".to_string(),
                    },
                ],
            ),
            (
                "SOL".to_string(),
                vec![
                    PriceRecord {
                        price: 145.99,
                        change_pct: 5.3,
                        volume: 3000.0,
                        recorded_at: "2024-03-02 12:00".to_string(),
                    },
                ],
            ),
        ];

        let messages = build_multi_prompt("detect divergence", &data);
        let user_msg = &messages[1].content;

        // Verify all BTC price records are present
        assert!(user_msg.contains("67000.50"), "should contain BTC price 67000.50");
        assert!(user_msg.contains("65500.75"), "should contain BTC price 65500.75");
        assert!(user_msg.contains("+2.1%"), "should contain BTC change_pct +2.1%");
        assert!(user_msg.contains("-1.8%"), "should contain BTC change_pct -1.8%");
        assert!(user_msg.contains("1200.5"), "should contain BTC volume 1200.5");
        assert!(user_msg.contains("900.3"), "should contain BTC volume 900.3");
        assert!(user_msg.contains("2024-03-02 12:00"), "should contain BTC timestamp");
        assert!(user_msg.contains("2024-03-01 12:00"), "should contain BTC timestamp");

        // Verify all ETH price records are present
        assert!(user_msg.contains("3500.25"), "should contain ETH price 3500.25");
        assert!(user_msg.contains("3450.00"), "should contain ETH price 3450.00");
        assert!(user_msg.contains("3460.10"), "should contain ETH price 3460.10");
        assert!(user_msg.contains("+0.9%"), "should contain ETH change_pct +0.9%");
        assert!(user_msg.contains("-0.3%"), "should contain ETH change_pct -0.3%");
        assert!(user_msg.contains("+1.2%"), "should contain ETH change_pct +1.2%");
        assert!(user_msg.contains("2100.0"), "should contain ETH volume 2100.0");
        assert!(user_msg.contains("1800.7"), "should contain ETH volume 1800.7");
        assert!(user_msg.contains("1500.0"), "should contain ETH volume 1500.0");
        assert!(user_msg.contains("2024-02-29 12:00"), "should contain ETH timestamp");

        // Verify SOL price record is present
        assert!(user_msg.contains("145.99"), "should contain SOL price 145.99");
        assert!(user_msg.contains("+5.3%"), "should contain SOL change_pct +5.3%");
        assert!(user_msg.contains("3000.0"), "should contain SOL volume 3000.0");
    }

    #[test]
    fn test_multi_prompt_completeness_contains_trigger_condition() {
        let condition = "BTC and ETH are both declining more than 3% while SOL is rising";
        let data = vec![
            (
                "BTC".to_string(),
                vec![PriceRecord {
                    price: 60000.0,
                    change_pct: -3.5,
                    volume: 1000.0,
                    recorded_at: "2024-03-01 10:00".to_string(),
                }],
            ),
            (
                "ETH".to_string(),
                vec![PriceRecord {
                    price: 3200.0,
                    change_pct: -4.0,
                    volume: 500.0,
                    recorded_at: "2024-03-01 10:00".to_string(),
                }],
            ),
            (
                "SOL".to_string(),
                vec![PriceRecord {
                    price: 150.0,
                    change_pct: 7.2,
                    volume: 2000.0,
                    recorded_at: "2024-03-01 10:00".to_string(),
                }],
            ),
        ];

        let messages = build_multi_prompt(condition, &data);
        let user_msg = &messages[1].content;

        // Trigger condition must appear in the user message
        assert!(
            user_msg.contains(condition),
            "user message should contain the full trigger condition text"
        );
    }
}
