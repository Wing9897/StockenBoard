//! 通知派發器
//!
//! 根據 channel_type 分派通知至對應的 sender（Telegram 或 Webhook），
//! 並記錄發送結果至 notification_history。

use std::sync::Arc;

use crate::db::DbPool;
use crate::notifications::models::{
    ChannelType, NotificationData, NotificationRule, TelegramConfig, WebhookConfig,
};
use crate::notifications::telegram::{format_telegram_message, send_telegram};
use crate::notifications::webhook::{build_webhook_payload, send_webhook};

/// 派發通知至規則綁定的所有通道
///
/// 對每個 channel_id：
/// 1. 從 DB 查詢通道設定
/// 2. 根據 channel_type 呼叫對應 sender
/// 3. 記錄成功/失敗至 notification_history
pub async fn dispatch_notification(
    db: &Arc<DbPool>,
    http_client: &reqwest::Client,
    rule: &NotificationRule,
    data: &NotificationData,
) {
    let channels = match db.list_notification_channels() {
        Ok(ch) => ch,
        Err(e) => {
            eprintln!("[Dispatcher] 無法載入通道列表: {}", e);
            return;
        }
    };

    for channel_id in &rule.channel_ids {
        let channel = match channels.iter().find(|c| c.id == *channel_id) {
            Some(c) => c,
            None => {
                eprintln!("[Dispatcher] 通道 {} 不存在，跳過", channel_id);
                continue;
            }
        };

        let channel_type = match ChannelType::from_str(&channel.channel_type) {
            Some(ct) => ct,
            None => {
                eprintln!(
                    "[Dispatcher] 通道 {} 類型無效: {}",
                    channel_id, channel.channel_type
                );
                continue;
            }
        };

        let result = match channel_type {
            ChannelType::Telegram => {
                let config: TelegramConfig = match serde_json::from_str(&channel.config) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("[Dispatcher] Telegram 設定解析失敗: {}", e);
                        let message = format_telegram_message(data);
                        record_history(
                            db,
                            rule.id,
                            *channel_id,
                            data.price,
                            "failed",
                            &message,
                            Some(&format!("Config parse error: {}", e)),
                        );
                        continue;
                    }
                };
                let message = format_telegram_message(data);
                let send_result = send_telegram(http_client, &config, &message).await;
                (send_result, message)
            }
            ChannelType::Webhook => {
                let config: WebhookConfig = match serde_json::from_str(&channel.config) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("[Dispatcher] Webhook 設定解析失敗: {}", e);
                        record_history(
                            db,
                            rule.id,
                            *channel_id,
                            data.price,
                            "failed",
                            "",
                            Some(&format!("Config parse error: {}", e)),
                        );
                        continue;
                    }
                };
                let send_result = send_webhook(http_client, &config, data).await;
                let payload_str =
                    serde_json::to_string(&build_webhook_payload(data)).unwrap_or_default();
                (send_result, payload_str)
            }
        };

        let (send_result, message) = result;
        match send_result {
            Ok(()) => {
                record_history(db, rule.id, *channel_id, data.price, "success", &message, None);
            }
            Err(e) => {
                eprintln!("[Dispatcher] 通道 {} 發送失敗: {}", channel_id, e);
                record_history(
                    db,
                    rule.id,
                    *channel_id,
                    data.price,
                    "failed",
                    &message,
                    Some(&e),
                );
            }
        }
    }
}

/// 記錄通知歷史
fn record_history(
    db: &Arc<DbPool>,
    rule_id: i64,
    channel_id: i64,
    price: f64,
    status: &str,
    message: &str,
    error: Option<&str>,
) {
    if let Err(e) = db.insert_notification_history(rule_id, channel_id, status, price, message, error) {
        eprintln!("[Dispatcher] 寫入通知歷史失敗: {}", e);
    }
}
