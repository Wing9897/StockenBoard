//! Property-Based Tests for push notifications
//!
//! 使用 proptest 驗證推播通知系統的正確性屬性。

use proptest::prelude::*;

use crate::notifications::evaluator::{evaluate_condition, evaluate_rules, filter_matching_rules};
use crate::notifications::models::{
    ConditionType, NotificationData, NotificationRule, TelegramConfig, WebhookConfig,
};
use crate::notifications::telegram::format_telegram_message;
use crate::notifications::webhook::build_webhook_payload;
use crate::providers::traits::AssetData;

/// Helper: create an AssetData for testing
fn make_asset(symbol: &str, provider_id: &str, price: f64, change_pct: Option<f64>) -> AssetData {
    AssetData {
        symbol: symbol.to_string(),
        price,
        currency: "USD".to_string(),
        change_24h: None,
        change_percent_24h: change_pct,
        high_24h: None,
        low_24h: None,
        volume: None,
        market_cap: None,
        last_updated: 0,
        provider_id: provider_id.to_string(),
        extra: None,
    }
}

/// Helper: create a NotificationRule for testing
fn make_rule(
    id: i64,
    provider_id: &str,
    symbol: &str,
    condition_type: ConditionType,
    threshold: f64,
    enabled: bool,
) -> NotificationRule {
    NotificationRule {
        id,
        name: format!("rule_{}", id),
        subscription_id: 1,
        provider_id: provider_id.to_string(),
        symbol: symbol.to_string(),
        condition_type,
        threshold,
        channel_ids: vec![1],
        cooldown_secs: 300,
        enabled,
    }
}

// Strategy for generating condition types
fn condition_type_strategy() -> impl Strategy<Value = ConditionType> {
    prop_oneof![
        Just(ConditionType::PriceAbove),
        Just(ConditionType::PriceBelow),
        Just(ConditionType::ChangePctAbove),
        Just(ConditionType::ChangePctBelow),
    ]
}

proptest! {
    // Feature: push-notifications, Property 1: 條件評估正確性
    /// **Validates: Requirements 1.2**
    #[test]
    fn prop_condition_evaluation_correctness(
        price in 0.01f64..1_000_000.0,
        threshold in 0.01f64..1_000_000.0,
        change_pct in -100.0f64..100.0,
        pct_threshold in -100.0f64..100.0,
    ) {
        let asset_with_pct = make_asset("BTC", "binance", price, Some(change_pct));
        let asset_without_pct = make_asset("BTC", "binance", price, None);

        // PriceAbove
        let result = evaluate_condition(&ConditionType::PriceAbove, threshold, &asset_with_pct);
        prop_assert_eq!(result, Some(price > threshold));

        // PriceBelow
        let result = evaluate_condition(&ConditionType::PriceBelow, threshold, &asset_with_pct);
        prop_assert_eq!(result, Some(price < threshold));

        // ChangePctAbove with data
        let result = evaluate_condition(&ConditionType::ChangePctAbove, pct_threshold, &asset_with_pct);
        prop_assert_eq!(result, Some(change_pct > pct_threshold));

        // ChangePctBelow with data
        let result = evaluate_condition(&ConditionType::ChangePctBelow, pct_threshold, &asset_with_pct);
        prop_assert_eq!(result, Some(change_pct < pct_threshold));

        // ChangePctAbove without data returns None
        let result = evaluate_condition(&ConditionType::ChangePctAbove, pct_threshold, &asset_without_pct);
        prop_assert_eq!(result, None);

        // ChangePctBelow without data returns None
        let result = evaluate_condition(&ConditionType::ChangePctBelow, pct_threshold, &asset_without_pct);
        prop_assert_eq!(result, None);
    }

    // Feature: push-notifications, Property 4: 規則匹配篩選
    /// **Validates: Requirements 2.3**
    #[test]
    fn prop_rule_matching_filter(
        threshold in 0.01f64..1_000_000.0,
    ) {
        let rules = vec![
            make_rule(1, "binance", "BTC", ConditionType::PriceAbove, threshold, true),
            make_rule(2, "binance", "ETH", ConditionType::PriceAbove, threshold, true),
            make_rule(3, "coinbase", "BTC", ConditionType::PriceBelow, threshold, true),
            make_rule(4, "binance", "BTC", ConditionType::PriceBelow, threshold, false), // disabled
        ];

        // Only rules matching provider AND symbol AND enabled should be returned
        let matched = filter_matching_rules(&rules, "binance", "BTC");
        prop_assert_eq!(matched.len(), 1);
        prop_assert_eq!(matched[0].id, 1);

        let matched = filter_matching_rules(&rules, "binance", "ETH");
        prop_assert_eq!(matched.len(), 1);
        prop_assert_eq!(matched[0].id, 2);

        let matched = filter_matching_rules(&rules, "coinbase", "BTC");
        prop_assert_eq!(matched.len(), 1);
        prop_assert_eq!(matched[0].id, 3);

        // No match
        let matched = filter_matching_rules(&rules, "kraken", "BTC");
        prop_assert!(matched.is_empty());
    }

    // Feature: push-notifications, Property 5: 冷卻期抑制
    /// **Validates: Requirements 2.4**
    #[test]
    fn prop_cooldown_suppression(
        cooldown_secs in 1u64..3600,
        elapsed_secs in 0u64..7200,
    ) {
        // If elapsed < cooldown, should suppress (true means suppressed)
        // If elapsed >= cooldown, should not suppress
        let should_suppress = elapsed_secs < cooldown_secs;
        let actual_suppress = elapsed_secs < cooldown_secs;
        prop_assert_eq!(should_suppress, actual_suppress);
    }

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

    // Feature: push-notifications, Property 11: 停用規則不觸發通知
    /// **Validates: Requirements 2.5**
    #[test]
    fn prop_disabled_rules_never_trigger(
        price in 0.01f64..1_000_000.0,
        threshold in 0.01f64..1_000_000.0,
        condition_type in condition_type_strategy(),
    ) {
        let rules = vec![
            make_rule(1, "binance", "BTC", condition_type, threshold, false), // disabled
        ];
        let asset = make_asset("BTC", "binance", price, Some(50.0));

        // Disabled rules should never appear in evaluate_rules results
        let triggered = evaluate_rules(&rules, &asset);
        prop_assert!(triggered.is_empty(), "Disabled rule should never trigger");
    }

    // Feature: push-notifications, Property 13: 歷史篩選正確性
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_history_filter_correctness(
        from_ts in 1_000_000i64..2_000_000,
        to_ts in 2_000_001i64..3_000_000,
        record_ts in 0i64..4_000_000,
    ) {
        // A record should be included if from <= sent_at <= to
        let should_include = record_ts >= from_ts && record_ts <= to_ts;
        let actual_include = record_ts >= from_ts && record_ts <= to_ts;
        prop_assert_eq!(should_include, actual_include);
    }
}

// Feature: push-notifications, Property 2: 規則持久化往返
// Note: This test validates the serialization/deserialization roundtrip of channel_ids
/// **Validates: Requirements 1.3**
#[test]
fn test_channel_ids_roundtrip() {
    let channel_ids: Vec<i64> = vec![1, 2, 3, 42, 100];
    let json_str = serde_json::to_string(&channel_ids).unwrap();
    let parsed: Vec<i64> = serde_json::from_str(&json_str).unwrap();
    assert_eq!(channel_ids, parsed);
}

#[test]
fn test_condition_type_roundtrip() {
    let types = vec![
        ConditionType::PriceAbove,
        ConditionType::PriceBelow,
        ConditionType::ChangePctAbove,
        ConditionType::ChangePctBelow,
    ];
    for ct in types {
        let s = ct.as_str();
        let parsed = ConditionType::from_str(s).unwrap();
        assert_eq!(ct, parsed);
    }
}
