//! Webhook 發送器
//!
//! 透過 HTTP POST 發送 JSON 格式的通知資料至使用者指定的 URL。
//! 支援自訂 Headers、10 秒逾時、重試邏輯（30 秒後重試一次）。

use super::models::{NotificationData, WebhookConfig};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;

/// 建構 Webhook JSON payload
///
/// 將 NotificationData 轉換為設計文件中定義的 JSON 格式：
/// ```json
/// {
///   "event": "price_alert",
///   "symbol": "BTC/USDT",
///   "provider": "binance",
///   "price": 67500.00,
///   "condition": "price_above",
///   "threshold": 65000.00,
///   "triggered_at": "2024-01-15T14:30:00Z",
///   "rule_name": "BTC 突破 65K"
/// }
/// ```
pub fn build_webhook_payload(data: &NotificationData) -> Value {
    json!({
        "event": "price_alert",
        "symbol": data.symbol,
        "provider": data.provider,
        "price": data.price,
        "condition": data.condition_type.as_str(),
        "threshold": data.threshold,
        "triggered_at": data.triggered_at.to_rfc3339(),
        "rule_name": data.rule_name,
    })
}

/// 透過 HTTP POST 發送 Webhook 通知
///
/// 建立 POST 請求至 `config.url`，設定 10 秒逾時。
/// 若 config.headers 有值，則加入自訂 HTTP Headers。
/// 若回應為 4xx/5xx 或請求失敗/逾時，等待 30 秒後重試一次。
/// 成功（2xx）回傳 Ok(())，最終失敗回傳 Err(String)。
pub async fn send_webhook(
    client: &reqwest::Client,
    config: &WebhookConfig,
    data: &NotificationData,
) -> Result<(), String> {
    let payload = build_webhook_payload(data);

    // First attempt
    match send_request(client, config, &payload).await {
        Ok(()) => return Ok(()),
        Err(e) => {
            eprintln!(
                "[Webhook] Request failed: {}. Retrying in 30 seconds...",
                e
            );
        }
    }

    // Wait 30 seconds before retry
    sleep(Duration::from_secs(30)).await;

    // Retry attempt
    send_request(client, config, &payload)
        .await
        .map_err(|e| format!("Webhook request failed after retry: {}", e))
}

/// 執行單次 HTTP POST 請求
async fn send_request(
    client: &reqwest::Client,
    config: &WebhookConfig,
    payload: &Value,
) -> Result<(), String> {
    let mut request = client
        .post(&config.url)
        .timeout(Duration::from_secs(10))
        .header("Content-Type", "application/json");

    // Add custom headers if present
    if let Some(headers) = &config.headers {
        for (key, value) in headers {
            request = request.header(key.as_str(), value.as_str());
        }
    }

    let result = request.json(payload).send().await;

    match result {
        Ok(response) if response.status().is_success() => Ok(()),
        Ok(response) => {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());
            Err(format!("HTTP {} - {}", status, error_text))
        }
        Err(e) => {
            if e.is_timeout() {
                Err(format!("Request timed out after 10 seconds: {}", e))
            } else {
                Err(format!("Request error: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notifications::models::ConditionType;
    use chrono::TimeZone;

    fn sample_notification_data() -> NotificationData {
        NotificationData {
            symbol: "BTC/USDT".to_string(),
            provider: "binance".to_string(),
            price: 67500.0,
            condition_type: ConditionType::PriceAbove,
            threshold: 65000.0,
            rule_name: "BTC 突破 65K".to_string(),
            triggered_at: chrono::Utc
                .with_ymd_and_hms(2024, 1, 15, 14, 30, 0)
                .unwrap(),
        }
    }

    #[test]
    fn test_build_webhook_payload_has_all_fields() {
        let data = sample_notification_data();
        let payload = build_webhook_payload(&data);

        assert_eq!(payload["event"], "price_alert");
        assert_eq!(payload["symbol"], "BTC/USDT");
        assert_eq!(payload["provider"], "binance");
        assert_eq!(payload["price"], 67500.0);
        assert_eq!(payload["condition"], "price_above");
        assert_eq!(payload["threshold"], 65000.0);
        assert_eq!(payload["rule_name"], "BTC 突破 65K");
        // triggered_at should be RFC 3339 format
        let triggered_at = payload["triggered_at"].as_str().unwrap();
        assert!(triggered_at.contains("2024-01-15"));
        assert!(triggered_at.contains("14:30:00"));
    }

    #[test]
    fn test_build_webhook_payload_condition_price_below() {
        let data = NotificationData {
            symbol: "ETH/USDT".to_string(),
            provider: "coinbase".to_string(),
            price: 2800.50,
            condition_type: ConditionType::PriceBelow,
            threshold: 3000.0,
            rule_name: "ETH 跌破 3K".to_string(),
            triggered_at: chrono::Utc
                .with_ymd_and_hms(2024, 3, 20, 8, 15, 0)
                .unwrap(),
        };

        let payload = build_webhook_payload(&data);

        assert_eq!(payload["event"], "price_alert");
        assert_eq!(payload["symbol"], "ETH/USDT");
        assert_eq!(payload["provider"], "coinbase");
        assert_eq!(payload["price"], 2800.50);
        assert_eq!(payload["condition"], "price_below");
        assert_eq!(payload["threshold"], 3000.0);
        assert_eq!(payload["rule_name"], "ETH 跌破 3K");
    }

    #[test]
    fn test_build_webhook_payload_condition_change_pct_above() {
        let data = NotificationData {
            symbol: "SOL/USDT".to_string(),
            provider: "binance".to_string(),
            price: 150.0,
            condition_type: ConditionType::ChangePctAbove,
            threshold: 10.0,
            rule_name: "SOL 漲幅超過 10%".to_string(),
            triggered_at: chrono::Utc
                .with_ymd_and_hms(2024, 2, 1, 12, 0, 0)
                .unwrap(),
        };

        let payload = build_webhook_payload(&data);

        assert_eq!(payload["condition"], "change_pct_above");
        assert_eq!(payload["threshold"], 10.0);
    }

    #[test]
    fn test_build_webhook_payload_condition_change_pct_below() {
        let data = NotificationData {
            symbol: "DOGE/USDT".to_string(),
            provider: "binance".to_string(),
            price: 0.08,
            condition_type: ConditionType::ChangePctBelow,
            threshold: -5.0,
            rule_name: "DOGE 跌幅超過 5%".to_string(),
            triggered_at: chrono::Utc
                .with_ymd_and_hms(2024, 4, 10, 6, 45, 30)
                .unwrap(),
        };

        let payload = build_webhook_payload(&data);

        assert_eq!(payload["condition"], "change_pct_below");
        assert_eq!(payload["threshold"], -5.0);
        assert_eq!(payload["price"], 0.08);
    }

    #[test]
    fn test_build_webhook_payload_triggered_at_is_rfc3339() {
        let data = sample_notification_data();
        let payload = build_webhook_payload(&data);

        let triggered_at = payload["triggered_at"].as_str().unwrap();
        // Should parse as a valid RFC 3339 / ISO 8601 datetime
        let parsed = chrono::DateTime::parse_from_rfc3339(triggered_at);
        assert!(
            parsed.is_ok(),
            "triggered_at should be valid RFC 3339: {}",
            triggered_at
        );
    }

    #[test]
    fn test_build_webhook_payload_no_extra_fields() {
        let data = sample_notification_data();
        let payload = build_webhook_payload(&data);

        let obj = payload.as_object().unwrap();
        assert_eq!(obj.len(), 8, "Payload should have exactly 8 fields");
        assert!(obj.contains_key("event"));
        assert!(obj.contains_key("symbol"));
        assert!(obj.contains_key("provider"));
        assert!(obj.contains_key("price"));
        assert!(obj.contains_key("condition"));
        assert!(obj.contains_key("threshold"));
        assert!(obj.contains_key("triggered_at"));
        assert!(obj.contains_key("rule_name"));
    }
}
