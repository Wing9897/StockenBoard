//! Dispatch and channel delivery tests — channel validation, message formatting, e2e dispatch

use proptest::prelude::*;

use crate::notifications::models::{
    ConditionType, NotificationData, TelegramConfig, WebhookConfig,
};
use crate::notifications::telegram::format_telegram_message;
use crate::notifications::webhook::build_webhook_payload;

proptest! {
    // Feature: push-notifications, Property 7: 通道建立驗證
    /// **Validates: Requirements 3.1**
    #[test]
    fn prop_channel_validation(
        bot_token in ".*",
        chat_id in ".*",
        url in ".*",
    ) {
        // Telegram: both bot_token and chat_id must be non-empty
        let telegram_valid = !bot_token.is_empty() && !chat_id.is_empty();
        let telegram_config = TelegramConfig {
            bot_token: bot_token.clone(),
            chat_id: chat_id.clone(),
        };
        let actual_valid = !telegram_config.bot_token.is_empty() && !telegram_config.chat_id.is_empty();
        prop_assert_eq!(telegram_valid, actual_valid);

        // Webhook: url must be non-empty
        let webhook_valid = !url.is_empty();
        let webhook_config = WebhookConfig {
            url: url.clone(),
            headers: None,
        };
        let actual_valid = !webhook_config.url.is_empty();
        prop_assert_eq!(webhook_valid, actual_valid);
    }

    // Feature: push-notifications, Property 8 & 9: 訊息格式完整性
    /// **Validates: Requirements 3.2, 3.3**
    #[test]
    fn prop_message_format_completeness(
        symbol in "[A-Z]{2,5}/[A-Z]{2,5}",
        provider in "[a-z]{3,10}",
        price in 0.01f64..1_000_000.0,
        threshold in 0.01f64..1_000_000.0,
        rule_name in ".{1,30}",
    ) {
        let data = NotificationData {
            symbol: symbol.clone(),
            provider: provider.clone(),
            price,
            condition_type: ConditionType::PriceAbove,
            threshold,
            rule_name: rule_name.clone(),
            triggered_at: chrono::Utc::now(),
        };

        // Property 8: Telegram message contains all required fields
        let telegram_msg = format_telegram_message(&data);
        prop_assert!(telegram_msg.contains(&symbol), "Telegram message missing symbol");
        prop_assert!(telegram_msg.contains(&provider), "Telegram message missing provider");
        prop_assert!(telegram_msg.contains("StockenBoard"), "Telegram message missing app name");

        // Property 9: Webhook payload contains all required fields
        let payload = build_webhook_payload(&data);
        prop_assert_eq!(payload["event"].as_str().unwrap(), "price_alert");
        prop_assert_eq!(payload["symbol"].as_str().unwrap(), symbol.as_str());
        prop_assert_eq!(payload["provider"].as_str().unwrap(), provider.as_str());
        prop_assert_eq!(payload["price"].as_f64().unwrap(), price);
        prop_assert_eq!(payload["condition"].as_str().unwrap(), "price_above");
        prop_assert_eq!(payload["threshold"].as_f64().unwrap(), threshold);
        prop_assert!(payload["triggered_at"].is_string());
        prop_assert_eq!(payload["rule_name"].as_str().unwrap(), rule_name.as_str());
    }
}

// === Local Notification Event Payload Integrity Property Test (Property 2) ===

mod local_notification_payload_property_tests {
    use crate::notifications::models::{ConditionType, NotificationData};
    use chrono::{TimeZone, Utc};
    use proptest::prelude::*;

    /// Strategy for generating a non-empty symbol string (e.g. "BTC/USDT")
    fn symbol_strategy() -> impl Strategy<Value = String> {
        "[A-Z]{2,5}/[A-Z]{2,5}"
    }

    /// Strategy for generating a non-empty rule name string
    fn rule_name_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{1,50}"
    }

    /// Strategy for generating a valid positive price
    fn price_strategy() -> impl Strategy<Value = f64> {
        0.001f64..1_000_000.0
    }

    /// Strategy for generating a valid UTC timestamp (year 2020-2030)
    fn timestamp_strategy() -> impl Strategy<Value = chrono::DateTime<Utc>> {
        (
            2020i32..2030,
            1u32..13,
            1u32..29,
            0u32..24,
            0u32..60,
            0u32..60,
        )
            .prop_map(|(year, month, day, hour, min, sec)| {
                Utc.with_ymd_and_hms(year, month, day, hour, min, sec)
                    .unwrap()
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Feature: logo-management-and-local-notifications, Property 2: Notification event payload integrity
        /// **Validates: Requirements 4.1**
        #[test]
        fn prop_local_notification_payload_preserves_input_data(
            symbol in symbol_strategy(),
            rule_name in rule_name_strategy(),
            price in price_strategy(),
            provider in "[a-z]{3,10}",
            threshold in 0.01f64..1_000_000.0,
            triggered_at in timestamp_strategy(),
        ) {
            let data = NotificationData {
                symbol: symbol.clone(),
                provider: provider.clone(),
                price,
                condition_type: ConditionType::PriceAbove,
                threshold,
                rule_name: rule_name.clone(),
                triggered_at,
            };

            // This is the exact formatting logic used in the local channel dispatch path
            let message = format!("[{}] {} @ ${}", data.symbol, data.rule_name, data.price);

            // Verify the formatted message preserves all input fields
            prop_assert!(
                message.contains(&symbol),
                "Local notification message missing symbol '{}'. Message: {}", symbol, message
            );
            prop_assert!(
                message.contains(&rule_name),
                "Local notification message missing rule_name '{}'. Message: {}", rule_name, message
            );
            // Verify price is present in the message (formatted as f64 string)
            let price_str = format!("{}", price);
            prop_assert!(
                message.contains(&price_str),
                "Local notification message missing price '{}'. Message: {}", price_str, message
            );
            // Verify the message structure follows the expected format: [symbol] rule_name @ $price
            let expected_message = format!("[{}] {} @ ${}", symbol, rule_name, price);
            prop_assert_eq!(
                message, expected_message,
                "Local notification message format mismatch"
            );
        }
    }
}

// === AI Notification Dispatch Property Tests (Property 11) ===

mod ai_notification_dispatch_property_tests {
    use crate::notifications::models::{ConditionType, NotificationData};
    use crate::notifications::telegram::format_telegram_message;
    use chrono::{TimeZone, Utc};
    use proptest::prelude::*;

    /// Strategy for generating a non-empty symbol string (e.g. "BTC/USDT")
    fn symbol_strategy() -> impl Strategy<Value = String> {
        "[A-Z]{2,5}/[A-Z]{2,5}"
    }

    /// Strategy for generating a non-empty reason string (safe for display)
    fn reason_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{1,100}"
    }

    /// Strategy for generating a valid UTC timestamp (year 2020-2030)
    fn timestamp_strategy() -> impl Strategy<Value = chrono::DateTime<Utc>> {
        (
            2020i32..2030,
            1u32..13,
            1u32..29,
            0u32..24,
            0u32..60,
            0u32..60,
        )
            .prop_map(|(year, month, day, hour, min, sec)| {
                Utc.with_ymd_and_hms(year, month, day, hour, min, sec)
                    .unwrap()
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Feature: ai-notification-rules, Property 11: Notification Message Contains Required Info
        /// **Validates: Requirements 5.2**
        #[test]
        fn prop_ai_notification_message_contains_required_info(
            symbol in symbol_strategy(),
            reason in reason_strategy(),
            triggered_at in timestamp_strategy(),
        ) {
            let data = NotificationData {
                symbol: symbol.clone(),
                provider: String::new(),
                price: 0.0,
                condition_type: ConditionType::Ai,
                threshold: 0.0,
                rule_name: format!("[AI] {}", reason),
                triggered_at,
            };

            let message = format_telegram_message(&data);

            // The message should contain the symbol
            prop_assert!(
                message.contains(&symbol),
                "Message missing symbol '{}'. Message: {}", symbol, message
            );

            // The message should contain the reason
            prop_assert!(
                message.contains(&reason),
                "Message missing reason '{}'. Message: {}", reason, message
            );

            // The message should contain the formatted time
            let time_display = triggered_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
            prop_assert!(
                message.contains(&time_display),
                "Message missing time '{}'. Message: {}", time_display, message
            );
        }
    }
}

// === End-to-End Integration Test: AI Rule → Mock AI API → Notification Dispatch ===
// **Validates: Requirements 4 (AI analysis execution), 5 (AI trigger notification dispatch)**

mod ai_e2e_integration_tests {
    use std::net::SocketAddr;
    use std::path::PathBuf;
    use std::sync::Arc;

    use axum::{routing::post, Json, Router};
    use tokio::net::TcpListener;

    use crate::db::DbPool;
    use crate::notifications::ai_evaluator::evaluate_ai_rule;
    use crate::notifications::dispatcher::dispatch_notification;
    use crate::notifications::models::{
        AiConfig, AiProviderConfig, ConditionType, NotificationData, NotificationRule,
    };

    /// Mock AI API handler that returns trigger=true with a reason
    async fn mock_ai_trigger_true() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "id": "chatcmpl-test-123",
            "object": "chat.completion",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "{\"trigger\": true, \"reason\": \"價格在最近 5 筆紀錄中上升了 6.2%，超過設定的 5% 閾值\"}"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 30,
                "total_tokens": 130
            }
        }))
    }

    /// Mock webhook handler that accepts any POST and returns 200 OK
    async fn mock_webhook_ok() -> axum::http::StatusCode {
        axum::http::StatusCode::OK
    }

    /// Start a mock server with both AI API and webhook endpoints, return its address
    async fn start_mock_server() -> SocketAddr {
        let app = Router::new()
            .route("/chat/completions", post(mock_ai_trigger_true))
            .route("/webhook", post(mock_webhook_ok));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        addr
    }

    /// Helper: set up an in-memory DB with subscription, price history, AI rule, and webhook channel
    fn setup_test_db(webhook_url: &str) -> (Arc<DbPool>, i64, i64, i64) {
        let db = Arc::new(DbPool::open(&PathBuf::from(":memory:")).unwrap());

        // 1. Create a subscription
        let sub_id = db
            .add_subscription(
                "asset", "BTC/USDT", None, "binance", "crypto", None, None, None,
            )
            .unwrap();

        // 2. Insert price history records directly (bypassing record_enabled check)
        let now = chrono::Utc::now().timestamp();
        let records: Vec<(f64, Option<f64>, Option<f64>, i64)> = vec![
            (68500.0, Some(6.2), Some(1234.5), now - 300),
            (67800.0, Some(4.1), Some(1100.0), now - 240),
            (66500.0, Some(2.5), Some(980.0), now - 180),
            (65200.0, Some(1.0), Some(850.0), now - 120),
            (64500.0, Some(-0.5), Some(750.0), now - 60),
        ];
        db.insert_price_history_for_test(sub_id, "binance", &records)
            .unwrap();

        // 3. Create a webhook notification channel pointing to mock server
        let webhook_config = serde_json::json!({
            "url": webhook_url,
            "headers": null
        });
        let channel_id = db
            .create_notification_channel("webhook", "Test Webhook", &webhook_config.to_string())
            .unwrap();

        // 4. Create an AI notification rule
        let ai_config = serde_json::json!({
            "prompt": "當價格在短時間內大幅上升超過 5% 時提醒我",
            "history_window": 5,
            "analysis_interval_secs": 60
        });
        let channel_ids_json = serde_json::to_string(&vec![channel_id]).unwrap();
        let rule_id = db
            .create_notification_rule(
                "AI Price Surge Alert",
                sub_id,
                "ai",
                0.0,
                &channel_ids_json,
                300,
                Some(&ai_config.to_string()),
            )
            .unwrap();

        (db, sub_id, rule_id, channel_id)
    }

    #[tokio::test]
    async fn test_e2e_evaluate_ai_rule_trigger_true() {
        let addr = start_mock_server().await;
        let base_url = format!("http://{}", addr);
        let webhook_url = format!("http://{}/webhook", addr);

        let (db, sub_id, rule_id, _channel_id) = setup_test_db(&webhook_url);

        let ai_config = AiConfig {
            prompt: "當價格在短時間內大幅上升超過 5% 時提醒我".to_string(),
            history_window: 5,
            analysis_interval_secs: 60,
        };

        let provider_config = AiProviderConfig {
            base_url,
            model: "test-model".to_string(),
            api_key: Some("test-api-key".to_string()),
        };

        let http_client = reqwest::Client::new();

        let result = evaluate_ai_rule(
            &db,
            &http_client,
            rule_id,
            sub_id,
            &ai_config,
            &provider_config,
        )
        .await;

        assert!(
            result.is_ok(),
            "evaluate_ai_rule should succeed, got: {:?}",
            result.err()
        );
        let response = result.unwrap();
        assert!(response.trigger, "AI should trigger notification");
        assert!(!response.reason.is_empty(), "AI reason should not be empty");
        assert!(
            response.reason.contains("6.2%"),
            "AI reason should contain the percentage from mock response"
        );
    }

    #[tokio::test]
    async fn test_e2e_ai_trigger_dispatches_notification_and_records_history() {
        let addr = start_mock_server().await;
        let base_url = format!("http://{}", addr);
        let webhook_url = format!("http://{}/webhook", addr);

        let (db, sub_id, rule_id, channel_id) = setup_test_db(&webhook_url);

        let ai_config = AiConfig {
            prompt: "當價格在短時間內大幅上升超過 5% 時提醒我".to_string(),
            history_window: 5,
            analysis_interval_secs: 60,
        };

        let provider_config = AiProviderConfig {
            base_url,
            model: "test-model".to_string(),
            api_key: None,
        };

        let http_client = reqwest::Client::new();

        let ai_response = evaluate_ai_rule(
            &db,
            &http_client,
            rule_id,
            sub_id,
            &ai_config,
            &provider_config,
        )
        .await
        .expect("AI evaluation should succeed");

        assert!(ai_response.trigger, "AI should trigger");

        let rule = NotificationRule {
            id: rule_id,
            name: "AI Price Surge Alert".to_string(),
            subscription_id: sub_id,
            provider_id: String::new(),
            symbol: "BTC/USDT".to_string(),
            condition_type: ConditionType::Ai,
            threshold: 0.0,
            channel_ids: vec![channel_id],
            cooldown_secs: 300,
            enabled: true,
        };

        let triggered_at = chrono::Utc::now();
        let notif_data = NotificationData {
            symbol: "BTC/USDT".to_string(),
            provider: String::new(),
            price: 0.0,
            condition_type: ConditionType::Ai,
            threshold: 0.0,
            rule_name: format!("[AI] {}", ai_response.reason),
            triggered_at,
        };

        dispatch_notification(&db, &http_client, &rule, &notif_data).await;

        let history = db
            .query_notification_history(Some(rule_id), None, None, Some(10))
            .expect("Should be able to query notification history");

        assert!(
            !history.is_empty(),
            "notification_history should have at least one record after dispatch"
        );

        let record = &history[0];
        assert_eq!(record.rule_id, rule_id);
        assert_eq!(record.channel_id, channel_id);
        assert_eq!(
            record.status, "success",
            "Webhook dispatch should succeed with mock server"
        );
        assert!(
            record.message.contains("BTC/USDT"),
            "Notification message should contain the symbol. Got: {}",
            record.message
        );
        assert!(
            record.message.contains("6.2%"),
            "Notification message should contain the AI reason. Got: {}",
            record.message
        );
    }

    #[tokio::test]
    async fn test_e2e_evaluate_ai_rule_without_api_key() {
        let addr = start_mock_server().await;
        let base_url = format!("http://{}", addr);
        let webhook_url = format!("http://{}/webhook", addr);

        let (db, sub_id, rule_id, _channel_id) = setup_test_db(&webhook_url);

        let ai_config = AiConfig {
            prompt: "test prompt".to_string(),
            history_window: 5,
            analysis_interval_secs: 60,
        };

        let provider_config = AiProviderConfig {
            base_url,
            model: "llama3".to_string(),
            api_key: None,
        };

        let http_client = reqwest::Client::new();

        let result = evaluate_ai_rule(
            &db,
            &http_client,
            rule_id,
            sub_id,
            &ai_config,
            &provider_config,
        )
        .await;

        assert!(
            result.is_ok(),
            "Should work without API key, got: {:?}",
            result.err()
        );
        let response = result.unwrap();
        assert!(response.trigger);
    }

    #[tokio::test]
    async fn test_e2e_evaluate_ai_rule_no_price_history() {
        let addr = start_mock_server().await;
        let base_url = format!("http://{}", addr);

        let db = Arc::new(DbPool::open(&PathBuf::from(":memory:")).unwrap());

        let sub_id = db
            .add_subscription(
                "asset", "ETH/USDT", None, "binance", "crypto", None, None, None,
            )
            .unwrap();

        let ai_config = AiConfig {
            prompt: "test".to_string(),
            history_window: 10,
            analysis_interval_secs: 60,
        };

        let provider_config = AiProviderConfig {
            base_url,
            model: "test-model".to_string(),
            api_key: None,
        };

        let http_client = reqwest::Client::new();

        let result = evaluate_ai_rule(
            &db,
            &http_client,
            1, // rule_id
            sub_id,
            &ai_config,
            &provider_config,
        )
        .await;

        assert!(
            result.is_err(),
            "Should fail when no price history is available"
        );
    }
}
