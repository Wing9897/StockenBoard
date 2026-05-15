//! Telegram Bot 發送器
//!
//! 透過 Telegram Bot API 的 sendMessage 方法發送通知訊息。
//! 包含訊息格式化與重試邏輯（30 秒後重試一次）。

use super::models::{ConditionType, NotificationData, TelegramConfig};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

/// 將條件類型轉換為中文描述，並格式化閾值
fn format_condition_description(condition_type: &ConditionType, threshold: f64) -> String {
    match condition_type {
        ConditionType::PriceAbove => format!("價格高於 {}", format_price(threshold)),
        ConditionType::PriceBelow => format!("價格低於 {}", format_price(threshold)),
        ConditionType::ChangePctAbove => format!("24h漲幅超過 {:.2}%", threshold),
        ConditionType::ChangePctBelow => format!("24h跌幅超過 {:.2}%", threshold),
    }
}

/// 格式化價格顯示（加入千分位逗號）
fn format_price(price: f64) -> String {
    let formatted = format!("{:.2}", price);
    let parts: Vec<&str> = formatted.split('.').collect();
    let integer_part = parts[0];
    let decimal_part = parts[1];

    let negative = integer_part.starts_with('-');
    let digits = if negative {
        &integer_part[1..]
    } else {
        integer_part
    };

    let with_commas: String = digits
        .chars()
        .rev()
        .enumerate()
        .map(|(i, c)| {
            if i > 0 && i % 3 == 0 {
                format!(",{}", c)
            } else {
                c.to_string()
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if negative {
        format!("-${}.{}", with_commas, decimal_part)
    } else {
        format!("${}.{}", with_commas, decimal_part)
    }
}

/// 格式化 Telegram 通知訊息
///
/// 將 NotificationData 格式化為預定義的訊息模板：
/// ```text
/// 📊 StockenBoard 價格警報
///
/// Symbol: BTC/USDT
/// Provider: binance
/// 當前價格: $67,500.00
/// 觸發條件: 價格高於 $65,000.00
/// 觸發時間: 2024-01-15 14:30:00 UTC
/// ```
pub fn format_telegram_message(data: &NotificationData) -> String {
    let condition_desc = format_condition_description(&data.condition_type, data.threshold);
    let price_display = format_price(data.price);
    let time_display = data.triggered_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();

    format!(
        "📊 StockenBoard 價格警報\n\n\
         Symbol: {}\n\
         Provider: {}\n\
         當前價格: {}\n\
         觸發條件: {}\n\
         觸發時間: {}",
        data.symbol, data.provider, price_display, condition_desc, time_display
    )
}

/// 透過 Telegram Bot API 發送訊息
///
/// 使用 `https://api.telegram.org/bot{token}/sendMessage` 端點發送訊息。
/// 若 API 回傳錯誤，等待 30 秒後重試一次。若重試仍失敗，回傳錯誤。
pub async fn send_telegram(
    client: &reqwest::Client,
    config: &TelegramConfig,
    message: &str,
) -> Result<(), String> {
    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        config.bot_token
    );

    let body = json!({
        "chat_id": config.chat_id,
        "text": message,
        "parse_mode": "HTML"
    });

    // First attempt
    match send_request(client, &url, &body).await {
        Ok(()) => return Ok(()),
        Err(e) => {
            eprintln!("[Telegram] {}. Retrying in 30 seconds...", e);
        }
    }

    // Wait 30 seconds before retry
    sleep(Duration::from_secs(30)).await;

    // Retry attempt
    send_request(client, &url, &body)
        .await
        .map_err(|e| format!("Telegram failed after retry: {}", e))
}

/// 執行單次 Telegram API 請求
async fn send_request(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<(), String> {
    match client.post(url).json(body).send().await {
        Ok(response) if response.status().is_success() => Ok(()),
        Ok(response) => {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());
            Err(format!("API error (status {}): {}", status, error_text))
        }
        Err(e) => Err(format!("Request failed: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_format_telegram_message_price_above() {
        let data = NotificationData {
            symbol: "BTC/USDT".to_string(),
            provider: "binance".to_string(),
            price: 67500.0,
            condition_type: ConditionType::PriceAbove,
            threshold: 65000.0,
            rule_name: "BTC 突破 65K".to_string(),
            triggered_at: chrono::Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap(),
        };

        let message = format_telegram_message(&data);

        assert!(message.contains("📊 StockenBoard 價格警報"));
        assert!(message.contains("Symbol: BTC/USDT"));
        assert!(message.contains("Provider: binance"));
        assert!(message.contains("當前價格: $67,500.00"));
        assert!(message.contains("觸發條件: 價格高於 $65,000.00"));
        assert!(message.contains("觸發時間: 2024-01-15 14:30:00 UTC"));
    }

    #[test]
    fn test_format_telegram_message_price_below() {
        let data = NotificationData {
            symbol: "ETH/USDT".to_string(),
            provider: "coinbase".to_string(),
            price: 2800.50,
            condition_type: ConditionType::PriceBelow,
            threshold: 3000.0,
            rule_name: "ETH 跌破 3K".to_string(),
            triggered_at: chrono::Utc.with_ymd_and_hms(2024, 3, 20, 8, 15, 0).unwrap(),
        };

        let message = format_telegram_message(&data);

        assert!(message.contains("Symbol: ETH/USDT"));
        assert!(message.contains("Provider: coinbase"));
        assert!(message.contains("當前價格: $2,800.50"));
        assert!(message.contains("觸發條件: 價格低於 $3,000.00"));
    }

    #[test]
    fn test_format_telegram_message_change_pct_above() {
        let data = NotificationData {
            symbol: "SOL/USDT".to_string(),
            provider: "binance".to_string(),
            price: 150.0,
            condition_type: ConditionType::ChangePctAbove,
            threshold: 10.0,
            rule_name: "SOL 漲幅超過 10%".to_string(),
            triggered_at: chrono::Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap(),
        };

        let message = format_telegram_message(&data);

        assert!(message.contains("觸發條件: 24h漲幅超過 10.00%"));
    }

    #[test]
    fn test_format_telegram_message_change_pct_below() {
        let data = NotificationData {
            symbol: "DOGE/USDT".to_string(),
            provider: "binance".to_string(),
            price: 0.08,
            condition_type: ConditionType::ChangePctBelow,
            threshold: -5.0,
            rule_name: "DOGE 跌幅超過 5%".to_string(),
            triggered_at: chrono::Utc.with_ymd_and_hms(2024, 4, 10, 6, 45, 30).unwrap(),
        };

        let message = format_telegram_message(&data);

        assert!(message.contains("當前價格: $0.08"));
        assert!(message.contains("觸發條件: 24h跌幅超過 -5.00%"));
    }

    #[test]
    fn test_format_price_with_commas() {
        assert_eq!(format_price(67500.0), "$67,500.00");
        assert_eq!(format_price(1000000.0), "$1,000,000.00");
        assert_eq!(format_price(100.0), "$100.00");
        assert_eq!(format_price(0.08), "$0.08");
        assert_eq!(format_price(1234567.89), "$1,234,567.89");
    }

    #[test]
    fn test_format_condition_description() {
        assert_eq!(
            format_condition_description(&ConditionType::PriceAbove, 65000.0),
            "價格高於 $65,000.00"
        );
        assert_eq!(
            format_condition_description(&ConditionType::PriceBelow, 3000.0),
            "價格低於 $3,000.00"
        );
        assert_eq!(
            format_condition_description(&ConditionType::ChangePctAbove, 10.0),
            "24h漲幅超過 10.00%"
        );
        assert_eq!(
            format_condition_description(&ConditionType::ChangePctBelow, -5.0),
            "24h跌幅超過 -5.00%"
        );
    }
}
