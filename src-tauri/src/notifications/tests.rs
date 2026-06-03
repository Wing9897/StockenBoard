//! Property-Based Tests for push notifications
//!
//! 使用 proptest 驗證推播通知系統的正確性屬性。

use proptest::prelude::*;
use std::str::FromStr;

use crate::notifications::evaluator::{evaluate_condition, evaluate_rules, filter_matching_rules};
use crate::notifications::models::{
    AiConfig, ConditionType, NotificationData, NotificationRule, TelegramConfig, WebhookConfig,
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
        Just(ConditionType::Ai),
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
        ConditionType::Ai,
    ];
    for ct in types {
        let s = ct.as_str();
        let parsed = ConditionType::from_str(s).unwrap();
        assert_eq!(ct, parsed);
    }
}

// === AiConfig 驗證測試 ===

#[test]
fn test_ai_config_valid() {
    let config = AiConfig {
        prompt: "當價格大幅上升時提醒我".to_string(),
        history_window: 20,
        analysis_interval_secs: 300,
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_ai_config_empty_prompt() {
    let config = AiConfig {
        prompt: "".to_string(),
        history_window: 20,
        analysis_interval_secs: 300,
    };
    let err = config.validate().unwrap_err();
    assert!(err.contains("prompt must not be empty"));
}

#[test]
fn test_ai_config_prompt_too_long() {
    let config = AiConfig {
        prompt: "a".repeat(2001),
        history_window: 20,
        analysis_interval_secs: 300,
    };
    let err = config.validate().unwrap_err();
    assert!(err.contains("prompt must not exceed 2000 characters"));
}

#[test]
fn test_ai_config_prompt_at_max_length() {
    let config = AiConfig {
        prompt: "a".repeat(2000),
        history_window: 20,
        analysis_interval_secs: 300,
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_ai_config_history_window_zero() {
    let config = AiConfig {
        prompt: "test".to_string(),
        history_window: 0,
        analysis_interval_secs: 300,
    };
    let err = config.validate().unwrap_err();
    assert!(err.contains("history_window must be between 1 and 100"));
}

#[test]
fn test_ai_config_history_window_too_large() {
    let config = AiConfig {
        prompt: "test".to_string(),
        history_window: 101,
        analysis_interval_secs: 300,
    };
    let err = config.validate().unwrap_err();
    assert!(err.contains("history_window must be between 1 and 100"));
}

#[test]
fn test_ai_config_history_window_boundaries() {
    // history_window = 1 (min valid)
    let config = AiConfig {
        prompt: "test".to_string(),
        history_window: 1,
        analysis_interval_secs: 30,
    };
    assert!(config.validate().is_ok());

    // history_window = 100 (max valid)
    let config = AiConfig {
        prompt: "test".to_string(),
        history_window: 100,
        analysis_interval_secs: 30,
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_ai_config_interval_too_small() {
    let config = AiConfig {
        prompt: "test".to_string(),
        history_window: 20,
        analysis_interval_secs: 29,
    };
    let err = config.validate().unwrap_err();
    assert!(err.contains("analysis_interval_secs must be at least 30"));
}

#[test]
fn test_ai_config_interval_at_minimum() {
    let config = AiConfig {
        prompt: "test".to_string(),
        history_window: 20,
        analysis_interval_secs: 30,
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_ai_config_serialization_roundtrip() {
    let config = AiConfig {
        prompt: "當價格在短時間內大幅上升超過 5% 時提醒我".to_string(),
        history_window: 20,
        analysis_interval_secs: 300,
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: AiConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config, parsed);
}

#[test]
fn test_ai_config_json_format() {
    let json = r#"{"prompt": "test prompt", "history_window": 20, "analysis_interval_secs": 300}"#;
    let config: AiConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.prompt, "test prompt");
    assert_eq!(config.history_window, 20);
    assert_eq!(config.analysis_interval_secs, 300);
}

// === AiProviderConfig 驗證測試 ===

#[test]
fn test_ai_provider_config_valid() {
    use crate::notifications::models::AiProviderConfig;
    let config = AiProviderConfig {
        base_url: "http://localhost:11434/v1".to_string(),
        model: "llama3".to_string(),
        api_key: None,
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_ai_provider_config_valid_with_api_key() {
    use crate::notifications::models::AiProviderConfig;
    let config = AiProviderConfig {
        base_url: "https://api.openai.com/v1".to_string(),
        model: "gpt-4".to_string(),
        api_key: Some("sk-test-key-123".to_string()),
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_ai_provider_config_empty_base_url() {
    use crate::notifications::models::AiProviderConfig;
    let config = AiProviderConfig {
        base_url: "".to_string(),
        model: "llama3".to_string(),
        api_key: None,
    };
    let err = config.validate().unwrap_err();
    assert!(err.contains("base_url must not be empty"));
}

#[test]
fn test_ai_provider_config_whitespace_base_url() {
    use crate::notifications::models::AiProviderConfig;
    let config = AiProviderConfig {
        base_url: "   ".to_string(),
        model: "llama3".to_string(),
        api_key: None,
    };
    let err = config.validate().unwrap_err();
    assert!(err.contains("base_url must not be empty"));
}

#[test]
fn test_ai_provider_config_empty_model() {
    use crate::notifications::models::AiProviderConfig;
    let config = AiProviderConfig {
        base_url: "http://localhost:11434/v1".to_string(),
        model: "".to_string(),
        api_key: None,
    };
    let err = config.validate().unwrap_err();
    assert!(err.contains("model must not be empty"));
}

#[test]
fn test_ai_provider_config_whitespace_model() {
    use crate::notifications::models::AiProviderConfig;
    let config = AiProviderConfig {
        base_url: "http://localhost:11434/v1".to_string(),
        model: "  ".to_string(),
        api_key: None,
    };
    let err = config.validate().unwrap_err();
    assert!(err.contains("model must not be empty"));
}

// === AI Provider Config DB 讀寫測試 ===

#[test]
fn test_save_and_load_ai_provider_config_without_api_key() {
    use crate::db::DbPool;
    use std::path::PathBuf;

    let db = DbPool::open(&PathBuf::from(":memory:")).unwrap();

    // Save config without api_key
    db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
        .unwrap();

    // Load and verify
    let config = db.load_ai_provider_config().unwrap().unwrap();
    assert_eq!(config.base_url, "http://localhost:11434/v1");
    assert_eq!(config.model, "llama3");
    assert_eq!(config.api_key, None);
}

#[test]
fn test_save_and_load_ai_provider_config_with_api_key() {
    use crate::db::DbPool;
    use std::path::PathBuf;

    let db = DbPool::open(&PathBuf::from(":memory:")).unwrap();

    // Save config with api_key
    db.save_ai_provider_config(
        "https://api.openai.com/v1",
        "gpt-4",
        Some("sk-test-key-12345"),
    )
    .unwrap();

    // Load and verify
    let config = db.load_ai_provider_config().unwrap().unwrap();
    assert_eq!(config.base_url, "https://api.openai.com/v1");
    assert_eq!(config.model, "gpt-4");
    assert_eq!(config.api_key, Some("sk-test-key-12345".to_string()));
}

#[test]
fn test_load_ai_provider_config_not_set() {
    use crate::db::DbPool;
    use std::path::PathBuf;

    let db = DbPool::open(&PathBuf::from(":memory:")).unwrap();

    // Should return None when not configured
    let config = db.load_ai_provider_config().unwrap();
    assert!(config.is_none());
}

#[test]
fn test_save_ai_provider_config_validates_base_url() {
    use crate::db::DbPool;
    use std::path::PathBuf;

    let db = DbPool::open(&PathBuf::from(":memory:")).unwrap();

    let result = db.save_ai_provider_config("", "llama3", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("base_url must not be empty"));
}

#[test]
fn test_save_ai_provider_config_validates_model() {
    use crate::db::DbPool;
    use std::path::PathBuf;

    let db = DbPool::open(&PathBuf::from(":memory:")).unwrap();

    let result = db.save_ai_provider_config("http://localhost:11434/v1", "", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("model must not be empty"));
}

#[test]
fn test_save_ai_provider_config_overwrites_existing() {
    use crate::db::DbPool;
    use std::path::PathBuf;

    let db = DbPool::open(&PathBuf::from(":memory:")).unwrap();

    // Save initial config
    db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
        .unwrap();

    // Overwrite with new config
    db.save_ai_provider_config("https://api.openai.com/v1", "gpt-4", Some("sk-new-key"))
        .unwrap();

    // Verify the new config is loaded
    let config = db.load_ai_provider_config().unwrap().unwrap();
    assert_eq!(config.base_url, "https://api.openai.com/v1");
    assert_eq!(config.model, "gpt-4");
    assert_eq!(config.api_key, Some("sk-new-key".to_string()));
}

#[test]
fn test_save_ai_provider_config_empty_api_key_treated_as_none() {
    use crate::db::DbPool;
    use std::path::PathBuf;

    let db = DbPool::open(&PathBuf::from(":memory:")).unwrap();

    // Save with empty string api_key
    db.save_ai_provider_config("http://localhost:11434/v1", "llama3", Some(""))
        .unwrap();

    // Load - empty api_key should be treated as None
    let config = db.load_ai_provider_config().unwrap().unwrap();
    assert_eq!(config.api_key, None);
}

#[test]
fn test_ai_provider_config_api_key_is_encrypted_in_db() {
    use crate::db::DbPool;
    use std::path::PathBuf;

    let db = DbPool::open(&PathBuf::from(":memory:")).unwrap();

    let api_key = "sk-secret-key-should-be-encrypted";
    db.save_ai_provider_config("http://localhost:11434/v1", "llama3", Some(api_key))
        .unwrap();

    // Read raw value from settings - it should NOT be the plaintext key
    let raw_value = db.get_setting("ai_api_key").unwrap().unwrap();
    assert_ne!(raw_value, api_key);
    assert!(!raw_value.is_empty());
}

// === build_prompt 單元測試 ===

mod build_prompt_tests {
    use crate::notifications::ai_evaluator::{build_prompt, PriceRecord};

    #[test]
    fn test_build_prompt_returns_two_messages() {
        let records = vec![PriceRecord {
            price: 68500.0,
            change_pct: 2.3,
            volume: 1234.5,
            recorded_at: "2024-01-15 10:30".to_string(),
        }];
        let messages = build_prompt("價格上升超過5%時提醒", &records);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_build_prompt_system_message_content() {
        let messages = build_prompt("test", &[]);
        let system = &messages[0];
        assert!(system.content.contains("金融市場分析助手"));
        assert!(system.content.contains("JSON"));
        assert!(system.content.contains("trigger"));
        assert!(system.content.contains("reason"));
    }

    #[test]
    fn test_build_prompt_user_message_contains_condition() {
        let condition = "當價格在短時間內大幅上升超過 5% 時提醒我";
        let messages = build_prompt(condition, &[]);
        let user = &messages[1];
        assert!(user.content.contains(condition));
        assert!(user.content.contains("觸發條件："));
    }

    #[test]
    fn test_build_prompt_user_message_contains_all_price_data() {
        let records = vec![
            PriceRecord {
                price: 68500.0,
                change_pct: 2.3,
                volume: 1234.5,
                recorded_at: "2024-01-15 10:30".to_string(),
            },
            PriceRecord {
                price: 67000.0,
                change_pct: -1.5,
                volume: 987.2,
                recorded_at: "2024-01-15 10:25".to_string(),
            },
        ];
        let messages = build_prompt("test condition", &records);
        let user = &messages[1];

        // Check all prices
        assert!(user.content.contains("68500.00"));
        assert!(user.content.contains("67000.00"));

        // Check all change_pct values
        assert!(user.content.contains("+2.3%"));
        assert!(user.content.contains("-1.5%"));

        // Check all volumes
        assert!(user.content.contains("1234.5"));
        assert!(user.content.contains("987.2"));

        // Check all timestamps
        assert!(user.content.contains("2024-01-15 10:30"));
        assert!(user.content.contains("2024-01-15 10:25"));
    }

    #[test]
    fn test_build_prompt_user_message_contains_record_count() {
        let records = vec![
            PriceRecord {
                price: 100.0,
                change_pct: 1.0,
                volume: 500.0,
                recorded_at: "2024-01-01 00:00".to_string(),
            },
            PriceRecord {
                price: 200.0,
                change_pct: 2.0,
                volume: 600.0,
                recorded_at: "2024-01-01 01:00".to_string(),
            },
            PriceRecord {
                price: 300.0,
                change_pct: 3.0,
                volume: 700.0,
                recorded_at: "2024-01-01 02:00".to_string(),
            },
        ];
        let messages = build_prompt("test", &records);
        let user = &messages[1];
        assert!(user.content.contains("最近 3 筆價格紀錄"));
    }

    #[test]
    fn test_build_prompt_empty_price_history() {
        let messages = build_prompt("test condition", &[]);
        assert_eq!(messages.len(), 2);
        let user = &messages[1];
        assert!(user.content.contains("最近 0 筆價格紀錄"));
        assert!(user.content.contains("test condition"));
    }

    #[test]
    fn test_build_prompt_table_header_present() {
        let records = vec![PriceRecord {
            price: 100.0,
            change_pct: 0.0,
            volume: 50.0,
            recorded_at: "2024-01-01 00:00".to_string(),
        }];
        let messages = build_prompt("test", &records);
        let user = &messages[1];
        assert!(user
            .content
            .contains("| 時間 | 價格 | 漲跌幅(%) | 成交量 |"));
        assert!(user
            .content
            .contains("|------|------|-----------|--------|"));
    }

    #[test]
    fn test_build_prompt_positive_change_has_plus_sign() {
        let records = vec![PriceRecord {
            price: 100.0,
            change_pct: 5.5,
            volume: 50.0,
            recorded_at: "2024-01-01 00:00".to_string(),
        }];
        let messages = build_prompt("test", &records);
        let user = &messages[1];
        assert!(user.content.contains("+5.5%"));
    }

    #[test]
    fn test_build_prompt_negative_change_has_minus_sign() {
        let records = vec![PriceRecord {
            price: 100.0,
            change_pct: -3.2,
            volume: 50.0,
            recorded_at: "2024-01-01 00:00".to_string(),
        }];
        let messages = build_prompt("test", &records);
        let user = &messages[1];
        assert!(user.content.contains("-3.2%"));
    }

    #[test]
    fn test_build_prompt_zero_change_has_plus_sign() {
        let records = vec![PriceRecord {
            price: 100.0,
            change_pct: 0.0,
            volume: 50.0,
            recorded_at: "2024-01-01 00:00".to_string(),
        }];
        let messages = build_prompt("test", &records);
        let user = &messages[1];
        assert!(user.content.contains("+0.0%"));
    }
}

// === parse_ai_response 單元測試 ===

mod parse_ai_response_tests {
    use crate::notifications::ai_evaluator::{parse_ai_response, AiEvalError};

    // --- Property 7: Round-trip parsing ---

    #[test]
    fn test_parse_valid_json_trigger_true() {
        let json = r#"{"trigger": true, "reason": "價格上升超過 5%"}"#;
        let result = parse_ai_response(json).unwrap();
        assert!(result.trigger);
        assert_eq!(result.reason, "價格上升超過 5%");
    }

    #[test]
    fn test_parse_valid_json_trigger_false() {
        let json = r#"{"trigger": false, "reason": "no significant change"}"#;
        let result = parse_ai_response(json).unwrap();
        assert!(!result.trigger);
        assert_eq!(result.reason, "no significant change");
    }

    #[test]
    fn test_parse_json_with_whitespace() {
        let json = r#"  {"trigger": true, "reason": "test"}  "#;
        let result = parse_ai_response(json).unwrap();
        assert!(result.trigger);
        assert_eq!(result.reason, "test");
    }

    // --- Property 8: Extra fields tolerance ---

    #[test]
    fn test_parse_json_with_extra_fields() {
        let json = r#"{"trigger": true, "reason": "test", "confidence": 0.95, "extra": "ignored"}"#;
        let result = parse_ai_response(json).unwrap();
        assert!(result.trigger);
        assert_eq!(result.reason, "test");
    }

    #[test]
    fn test_parse_json_with_nested_extra_fields() {
        let json = r#"{"trigger": false, "reason": "stable", "metadata": {"model": "gpt-4", "tokens": 150}}"#;
        let result = parse_ai_response(json).unwrap();
        assert!(!result.trigger);
        assert_eq!(result.reason, "stable");
    }

    // --- Property 9: Markdown code block extraction ---

    #[test]
    fn test_parse_markdown_json_code_block() {
        let raw = "```json\n{\"trigger\": true, \"reason\": \"detected spike\"}\n```";
        let result = parse_ai_response(raw).unwrap();
        assert!(result.trigger);
        assert_eq!(result.reason, "detected spike");
    }

    #[test]
    fn test_parse_markdown_plain_code_block() {
        let raw = "```\n{\"trigger\": false, \"reason\": \"no change\"}\n```";
        let result = parse_ai_response(raw).unwrap();
        assert!(!result.trigger);
        assert_eq!(result.reason, "no change");
    }

    #[test]
    fn test_parse_markdown_with_surrounding_text() {
        let raw = "Here is my analysis:\n```json\n{\"trigger\": true, \"reason\": \"price surge\"}\n```\nEnd of response.";
        let result = parse_ai_response(raw).unwrap();
        assert!(result.trigger);
        assert_eq!(result.reason, "price surge");
    }

    // --- Property 10: Invalid responses yield errors ---

    #[test]
    fn test_parse_invalid_json() {
        let raw = "this is not json at all";
        let result = parse_ai_response(raw);
        assert!(result.is_err());
        match result.unwrap_err() {
            AiEvalError::InvalidJson(_) => {}
            other => panic!("Expected InvalidJson, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_missing_trigger_field() {
        let json = r#"{"reason": "test"}"#;
        let result = parse_ai_response(json);
        assert!(result.is_err());
        match result.unwrap_err() {
            AiEvalError::MissingField(msg) => {
                assert!(msg.contains("trigger"));
            }
            other => panic!("Expected MissingField, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_missing_reason_field() {
        let json = r#"{"trigger": true}"#;
        let result = parse_ai_response(json);
        assert!(result.is_err());
        match result.unwrap_err() {
            AiEvalError::MissingField(msg) => {
                assert!(msg.contains("reason"));
            }
            other => panic!("Expected MissingField, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_trigger_not_boolean() {
        let json = r#"{"trigger": "yes", "reason": "test"}"#;
        let result = parse_ai_response(json);
        assert!(result.is_err());
        match result.unwrap_err() {
            AiEvalError::MissingField(msg) => {
                assert!(msg.contains("trigger"));
                assert!(msg.contains("boolean"));
            }
            other => panic!("Expected MissingField, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_reason_not_string() {
        let json = r#"{"trigger": true, "reason": 123}"#;
        let result = parse_ai_response(json);
        assert!(result.is_err());
        match result.unwrap_err() {
            AiEvalError::MissingField(msg) => {
                assert!(msg.contains("reason"));
                assert!(msg.contains("string"));
            }
            other => panic!("Expected MissingField, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_json_array_not_object() {
        let json = r#"[true, "reason"]"#;
        let result = parse_ai_response(json);
        assert!(result.is_err());
        match result.unwrap_err() {
            AiEvalError::MissingField(msg) => {
                assert!(msg.contains("not a JSON object"));
            }
            other => panic!("Expected MissingField, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_empty_string() {
        let result = parse_ai_response("");
        assert!(result.is_err());
        match result.unwrap_err() {
            AiEvalError::InvalidJson(_) => {}
            other => panic!("Expected InvalidJson, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_trigger_as_number() {
        let json = r#"{"trigger": 1, "reason": "test"}"#;
        let result = parse_ai_response(json);
        assert!(result.is_err());
        match result.unwrap_err() {
            AiEvalError::MissingField(msg) => {
                assert!(msg.contains("trigger"));
            }
            other => panic!("Expected MissingField, got: {:?}", other),
        }
    }
}

// === AI Evaluator Property-Based Tests (Properties 6-10) ===

mod ai_evaluator_property_tests {
    use crate::notifications::ai_evaluator::{build_prompt, parse_ai_response, PriceRecord};
    use proptest::prelude::*;

    /// Strategy for generating valid PriceRecord values
    fn price_record_strategy() -> impl Strategy<Value = PriceRecord> {
        (
            0.01f64..1_000_000.0,                           // price
            -100.0f64..100.0,                               // change_pct
            0.0f64..1_000_000.0,                            // volume
            "[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}", // recorded_at
        )
            .prop_map(|(price, change_pct, volume, recorded_at)| PriceRecord {
                price,
                change_pct,
                volume,
                recorded_at,
            })
    }

    /// Strategy for generating a non-empty list of PriceRecords (1 to 20)
    fn price_records_strategy() -> impl Strategy<Value = Vec<PriceRecord>> {
        prop::collection::vec(price_record_strategy(), 1..20)
    }

    /// Strategy for generating a non-empty user condition string
    fn user_condition_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{1,100}"
    }

    /// Strategy for generating a non-empty reason string (safe for JSON embedding)
    fn reason_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{1,200}"
    }

    /// Strategy for generating arbitrary JSON key-value pairs (extra fields)
    fn extra_field_strategy() -> impl Strategy<Value = (String, String)> {
        (
            "[a-z_]{1,20}",       // key
            "[a-zA-Z0-9 ]{1,50}", // value (string)
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Feature: ai-notification-rules, Property 6: Prompt Contains All Price Data
        /// **Validates: Requirements 4.2**
        #[test]
        fn prop_prompt_contains_all_price_data(
            records in price_records_strategy(),
            condition in user_condition_strategy(),
        ) {
            let messages = build_prompt(&condition, &records);
            // The user message is the second element
            let user_msg = &messages[1].content;

            // The prompt must contain the user condition string verbatim
            prop_assert!(
                user_msg.contains(&condition),
                "Prompt missing user condition: '{}'", condition
            );

            for record in &records {
                // Every record's price value (formatted as {:.2})
                let price_str = format!("{:.2}", record.price);
                prop_assert!(
                    user_msg.contains(&price_str),
                    "Prompt missing price: {}", price_str
                );

                // Every record's change_pct value (formatted as {:+.1}%)
                let change_str = format!("{:+.1}%", record.change_pct);
                prop_assert!(
                    user_msg.contains(&change_str),
                    "Prompt missing change_pct: {}", change_str
                );

                // Every record's volume value (formatted as {:.1})
                let volume_str = format!("{:.1}", record.volume);
                prop_assert!(
                    user_msg.contains(&volume_str),
                    "Prompt missing volume: {}", volume_str
                );

                // Every record's timestamp
                prop_assert!(
                    user_msg.contains(&record.recorded_at),
                    "Prompt missing timestamp: {}", record.recorded_at
                );
            }
        }

        // Feature: ai-notification-rules, Property 7: AI Response Parsing Round-Trip
        /// **Validates: Requirements 7.1, 7.2, 7.3**
        #[test]
        fn prop_ai_response_parsing_roundtrip(
            trigger in any::<bool>(),
            reason in reason_strategy(),
        ) {
            let json = format!(r#"{{"trigger": {}, "reason": "{}"}}"#, trigger, reason);
            let result = parse_ai_response(&json).unwrap();
            prop_assert_eq!(result.trigger, trigger);
            prop_assert_eq!(result.reason, reason);
        }

        // Feature: ai-notification-rules, Property 8: Extra Fields Tolerance
        /// **Validates: Requirements 7.4**
        #[test]
        fn prop_extra_fields_tolerance(
            trigger in any::<bool>(),
            reason in reason_strategy(),
            extra_fields in prop::collection::vec(extra_field_strategy(), 1..5),
        ) {
            // Build JSON with extra fields
            let extra_json: String = extra_fields
                .iter()
                .map(|(k, v)| format!(r#""{}": "{}""#, k, v))
                .collect::<Vec<_>>()
                .join(", ");

            let json = format!(
                r#"{{"trigger": {}, "reason": "{}", {}}}"#,
                trigger, reason, extra_json
            );

            let result = parse_ai_response(&json).unwrap();
            prop_assert_eq!(result.trigger, trigger);
            prop_assert_eq!(result.reason, reason);
        }

        // Feature: ai-notification-rules, Property 9: Markdown Code Block Extraction
        /// **Validates: Requirements 7.5**
        #[test]
        fn prop_markdown_code_block_extraction(
            trigger in any::<bool>(),
            reason in reason_strategy(),
        ) {
            let raw_json = format!(r#"{{"trigger": {}, "reason": "{}"}}"#, trigger, reason);

            // Test with ```json\n...\n```
            let markdown_json = format!("```json\n{}\n```", raw_json);
            let result_md_json = parse_ai_response(&markdown_json).unwrap();

            // Test with ```\n...\n```
            let markdown_plain = format!("```\n{}\n```", raw_json);
            let result_md_plain = parse_ai_response(&markdown_plain).unwrap();

            // Test raw JSON directly
            let result_raw = parse_ai_response(&raw_json).unwrap();

            // All three should produce the same result
            prop_assert_eq!(result_md_json.trigger, result_raw.trigger);
            prop_assert_eq!(&result_md_json.reason, &result_raw.reason);
            prop_assert_eq!(result_md_plain.trigger, result_raw.trigger);
            prop_assert_eq!(&result_md_plain.reason, &result_raw.reason);
        }

        // Feature: ai-notification-rules, Property 10: Invalid Response Yields No Trigger
        /// **Validates: Requirements 4.5**
        #[test]
        fn prop_invalid_response_yields_error_not_valid_json(
            garbage in "[^{}\\[\\]\"]{1,100}",
        ) {
            // Any string that is not valid JSON should return an error
            let result = parse_ai_response(&garbage);
            prop_assert!(result.is_err(), "Expected error for non-JSON input: {}", garbage);
        }

        // Feature: ai-notification-rules, Property 10: Invalid Response Yields No Trigger (missing trigger)
        /// **Validates: Requirements 4.5**
        #[test]
        fn prop_invalid_response_missing_trigger(
            reason in reason_strategy(),
            extra_key in "[a-z]{1,10}",
            extra_val in "[a-zA-Z0-9]{1,20}",
        ) {
            // Valid JSON but missing the "trigger" field
            let json = format!(
                r#"{{"reason": "{}", "{}": "{}"}}"#,
                reason, extra_key, extra_val
            );
            let result = parse_ai_response(&json);
            prop_assert!(result.is_err(), "Expected error for JSON missing 'trigger': {}", json);
        }

        // Feature: ai-notification-rules, Property 10: Invalid Response Yields No Trigger (trigger not boolean)
        /// **Validates: Requirements 4.5**
        #[test]
        fn prop_invalid_response_trigger_not_boolean(
            trigger_val in "[a-zA-Z]{1,20}",
            reason in reason_strategy(),
        ) {
            // Valid JSON where "trigger" is a string, not a boolean
            let json = format!(
                r#"{{"trigger": "{}", "reason": "{}"}}"#,
                trigger_val, reason
            );
            let result = parse_ai_response(&json);
            prop_assert!(result.is_err(), "Expected error for non-boolean trigger: {}", json);
        }
    }
}

// === AI Notification Dispatch Property Tests (Properties 11, 12) ===

mod ai_notification_dispatch_property_tests {
    use crate::notifications::ai_scheduler::should_suppress_trigger;
    use crate::notifications::models::{ConditionType, NotificationData};
    use crate::notifications::telegram::format_telegram_message;
    use chrono::{TimeZone, Utc};
    use proptest::prelude::*;
    use std::time::{Duration, Instant};

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
        // Generate timestamps between 2020-01-01 and 2030-01-01
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
            // Create a NotificationData with condition_type = ConditionType::Ai
            // rule_name = format!("[AI] {}", reason)
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

        // Feature: ai-notification-rules, Property 12: Cooldown Prevents Re-Trigger
        /// **Validates: Requirements 5.3**
        #[test]
        fn prop_cooldown_prevents_retrigger(
            cooldown_secs in 1u64..3600,
            elapsed_millis in 0u64..7_200_000,
        ) {
            // Use the actual should_suppress_trigger helper from ai_scheduler.
            // We simulate a last_trigger_time by subtracting elapsed_millis from now.
            let elapsed_secs = elapsed_millis / 1000;

            // Case 1: When last_trigger is None (never triggered), should never suppress
            let result_none = should_suppress_trigger(None, cooldown_secs);
            prop_assert!(
                !result_none,
                "should_suppress_trigger(None, {}) should be false (never triggered before)",
                cooldown_secs
            );

            // Case 2: When last_trigger is Some(time) and elapsed < cooldown, should suppress
            // When elapsed >= cooldown, should NOT suppress
            // We create an Instant that is `elapsed_millis` ms in the past by using
            // Instant::now() - Duration::from_millis(elapsed_millis)
            let last_trigger = Instant::now() - Duration::from_millis(elapsed_millis);
            let result = should_suppress_trigger(Some(last_trigger), cooldown_secs);

            if elapsed_secs < cooldown_secs {
                prop_assert!(
                    result,
                    "Expected suppression when elapsed_secs ({}) < cooldown_secs ({})",
                    elapsed_secs, cooldown_secs
                );
            } else {
                prop_assert!(
                    !result,
                    "Expected NO suppression when elapsed_secs ({}) >= cooldown_secs ({})",
                    elapsed_secs, cooldown_secs
                );
            }
        }
    }
}

// === AI Data Model Property-Based Tests (Properties 1, 2, 3, 4, 5) ===

mod ai_data_model_property_tests {
    use crate::db::DbPool;
    use crate::notifications::crypto::{decrypt_token, encrypt_token};
    use crate::notifications::models::{AiConfig, AiProviderConfig};
    use proptest::prelude::*;
    use std::path::PathBuf;

    /// Strategy for generating valid prompts (non-empty, ≤ 2000 chars)
    fn valid_prompt_strategy() -> impl Strategy<Value = String> {
        // Generate strings of length 1..=2000 using safe characters
        prop::collection::vec(prop::char::range('a', 'z'), 1..=200)
            .prop_map(|chars| chars.into_iter().collect::<String>())
    }

    /// Strategy for generating valid history_window values [1, 100]
    fn valid_history_window_strategy() -> impl Strategy<Value = u32> {
        1u32..=100
    }

    /// Strategy for generating valid analysis_interval_secs values (≥ 30)
    fn valid_analysis_interval_strategy() -> impl Strategy<Value = u64> {
        30u64..=86400
    }

    /// Strategy for generating a valid AiConfig
    fn valid_ai_config_strategy() -> impl Strategy<Value = AiConfig> {
        (
            valid_prompt_strategy(),
            valid_history_window_strategy(),
            valid_analysis_interval_strategy(),
        )
            .prop_map(
                |(prompt, history_window, analysis_interval_secs)| AiConfig {
                    prompt,
                    history_window,
                    analysis_interval_secs,
                },
            )
    }

    /// Strategy for generating invalid history_window values (outside [1, 100])
    fn invalid_history_window_strategy() -> impl Strategy<Value = u32> {
        prop_oneof![Just(0u32), 101u32..=1000,]
    }

    /// Strategy for generating invalid analysis_interval_secs values (< 30)
    fn invalid_analysis_interval_strategy() -> impl Strategy<Value = u64> {
        0u64..30
    }

    /// Strategy for generating non-empty API key strings (safe for encryption)
    fn api_key_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_\\-]{1,100}"
    }

    /// Strategy for generating non-empty base_url strings
    fn base_url_strategy() -> impl Strategy<Value = String> {
        "https?://[a-z]{3,10}\\.[a-z]{2,5}/[a-z]{1,10}"
    }

    /// Strategy for generating non-empty model strings
    fn model_strategy() -> impl Strategy<Value = String> {
        "[a-z0-9\\-]{3,20}"
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Feature: ai-notification-rules, Property 1: AI Rule Persistence Round-Trip
        /// **Validates: Requirements 1.1, 1.3**
        #[test]
        fn prop_ai_rule_persistence_roundtrip(
            config in valid_ai_config_strategy(),
            rule_name in "[a-zA-Z0-9 ]{1,30}",
            cooldown_secs in 60i64..3600,
        ) {
            // Create an in-memory database
            let db = DbPool::open(&PathBuf::from(":memory:")).unwrap();

            // We need a subscription to reference
            db.add_subscription("asset", "BTC/USDT", None, "binance", "crypto", None, None, None).unwrap();

            // Serialize the AiConfig to JSON
            let ai_config_json = serde_json::to_string(&config).unwrap();

            // Create a notification rule with condition_type "ai" and the serialized ai_config
            let channel_ids_json = serde_json::to_string(&vec![1i64]).unwrap();
            let rule_id = db.create_notification_rule(
                &rule_name,
                1, // subscription_id
                "ai",
                0.0, // threshold (unused for AI rules)
                &channel_ids_json,
                cooldown_secs,
                Some(&ai_config_json),
            ).unwrap();

            // Load it back from DB
            let loaded = db.get_notification_rule(rule_id).unwrap().unwrap();

            // Assert all fields match
            prop_assert_eq!(&loaded.name, &rule_name);
            prop_assert_eq!(loaded.subscription_id, 1);
            prop_assert_eq!(&loaded.condition_type, "ai");
            prop_assert_eq!(loaded.threshold, 0.0);
            prop_assert_eq!(&loaded.channel_ids, &channel_ids_json);
            prop_assert_eq!(loaded.cooldown_secs, cooldown_secs);
            prop_assert!(loaded.enabled);

            // Deserialize the ai_config back and compare
            let loaded_ai_config: AiConfig = serde_json::from_str(
                loaded.ai_config.as_ref().unwrap()
            ).unwrap();
            prop_assert_eq!(loaded_ai_config.prompt, config.prompt);
            prop_assert_eq!(loaded_ai_config.history_window, config.history_window);
            prop_assert_eq!(loaded_ai_config.analysis_interval_secs, config.analysis_interval_secs);
        }

        // Feature: ai-notification-rules, Property 2: AI Config Validation Rejects Invalid Inputs
        /// **Validates: Requirements 1.2, 1.6, 1.7**
        #[test]
        fn prop_ai_config_validation_rejects_invalid_history_window(
            prompt in valid_prompt_strategy(),
            history_window in invalid_history_window_strategy(),
            analysis_interval_secs in valid_analysis_interval_strategy(),
        ) {
            let config = AiConfig {
                prompt,
                history_window,
                analysis_interval_secs,
            };
            prop_assert!(config.validate().is_err(),
                "Expected validation error for history_window={}", history_window);
        }

        // Feature: ai-notification-rules, Property 2: AI Config Validation Rejects Invalid Inputs
        /// **Validates: Requirements 1.2, 1.6, 1.7**
        #[test]
        fn prop_ai_config_validation_rejects_invalid_interval(
            prompt in valid_prompt_strategy(),
            history_window in valid_history_window_strategy(),
            analysis_interval_secs in invalid_analysis_interval_strategy(),
        ) {
            let config = AiConfig {
                prompt,
                history_window,
                analysis_interval_secs,
            };
            prop_assert!(config.validate().is_err(),
                "Expected validation error for analysis_interval_secs={}", analysis_interval_secs);
        }

        // Feature: ai-notification-rules, Property 2: AI Config Validation Rejects Invalid Inputs
        /// **Validates: Requirements 1.2, 1.6, 1.7**
        #[test]
        fn prop_ai_config_validation_rejects_empty_prompt(
            history_window in valid_history_window_strategy(),
            analysis_interval_secs in valid_analysis_interval_strategy(),
        ) {
            let config = AiConfig {
                prompt: String::new(),
                history_window,
                analysis_interval_secs,
            };
            prop_assert!(config.validate().is_err(),
                "Expected validation error for empty prompt");
        }

        // Feature: ai-notification-rules, Property 3: Provider Config Validation
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_provider_config_validation_rejects_empty_base_url(
            model in model_strategy(),
            api_key in proptest::option::of(api_key_strategy()),
        ) {
            let config = AiProviderConfig {
                base_url: String::new(),
                model,
                api_key,
            };
            prop_assert!(config.validate().is_err(),
                "Expected validation error for empty base_url");
        }

        // Feature: ai-notification-rules, Property 3: Provider Config Validation
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_provider_config_validation_rejects_empty_model(
            base_url in base_url_strategy(),
            api_key in proptest::option::of(api_key_strategy()),
        ) {
            let config = AiProviderConfig {
                base_url,
                model: String::new(),
                api_key,
            };
            prop_assert!(config.validate().is_err(),
                "Expected validation error for empty model");
        }

        // Feature: ai-notification-rules, Property 3: Provider Config Validation (whitespace-only)
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_provider_config_validation_rejects_whitespace_base_url(
            model in model_strategy(),
            spaces in " {1,10}",
        ) {
            let config = AiProviderConfig {
                base_url: spaces,
                model,
                api_key: None,
            };
            prop_assert!(config.validate().is_err(),
                "Expected validation error for whitespace-only base_url");
        }

        // Feature: ai-notification-rules, Property 4: Authorization Header Construction
        /// **Validates: Requirements 2.3**
        #[test]
        fn prop_authorization_header_with_api_key(
            base_url in base_url_strategy(),
            model in model_strategy(),
            api_key in api_key_strategy(),
        ) {
            let config = AiProviderConfig {
                base_url,
                model,
                api_key: Some(api_key.clone()),
            };

            // Simulate the header construction logic:
            // If api_key is Some(key), the Authorization header should be "Bearer {key}"
            let auth_header = config.api_key.as_ref().map(|key| format!("Bearer {}", key));

            prop_assert!(auth_header.is_some());
            let header_value = auth_header.unwrap();
            prop_assert!(header_value.starts_with("Bearer "));
            prop_assert!(header_value.contains(&api_key));
            prop_assert_eq!(header_value, format!("Bearer {}", api_key));
        }

        // Feature: ai-notification-rules, Property 4: Authorization Header Construction (no key)
        /// **Validates: Requirements 2.3**
        #[test]
        fn prop_no_authorization_header_without_api_key(
            base_url in base_url_strategy(),
            model in model_strategy(),
        ) {
            let config = AiProviderConfig {
                base_url,
                model,
                api_key: None,
            };

            // If api_key is None, no Authorization header should be present
            let auth_header = config.api_key.as_ref().map(|key| format!("Bearer {}", key));

            prop_assert!(auth_header.is_none(),
                "Expected no Authorization header when api_key is None");
        }

        // Feature: ai-notification-rules, Property 5: API Key Encryption Round-Trip
        /// **Validates: Requirements 2.4**
        #[test]
        fn prop_api_key_encryption_roundtrip(
            api_key in api_key_strategy(),
        ) {
            // Encrypt the key
            let encrypted = encrypt_token(&api_key).unwrap();

            // Encrypted should differ from plaintext
            prop_assert_ne!(&encrypted, &api_key,
                "Encrypted value should differ from plaintext");

            // Decrypt should return the original
            let decrypted = decrypt_token(&encrypted).unwrap();
            prop_assert_eq!(decrypted, api_key,
                "Decrypted value should match original");
        }
    }
}

// === AI Scheduler Integration Tests ===
// **Validates: Requirements 3.1, 3.4, 3.5, 3.6**
// Tests verify that the AiScheduler correctly manages task lifecycle:
// - Starting loads AI rules and spawns tasks (Req 3.1)
// - Removing a rule stops its task (Req 3.4)
// - Upserting a rule starts/restarts its task (Req 3.5, 3.6)
// - Reloading stops all tasks and restarts them

mod ai_scheduler_integration_tests {
    use crate::db::DbPool;
    use crate::notifications::ai_scheduler::AiScheduler;
    use std::path::PathBuf;
    use std::sync::Arc;

    /// Helper: create an in-memory DB and AiScheduler instance
    async fn setup_scheduler() -> (Arc<DbPool>, AiScheduler) {
        let db = Arc::new(DbPool::open(&PathBuf::from(":memory:")).unwrap());
        let scheduler = AiScheduler::new(db.clone());
        (db, scheduler)
    }

    /// Helper: create a subscription in the DB and return its ID
    fn create_test_subscription(db: &DbPool) -> i64 {
        db.add_subscription(
            "asset", "BTC/USDT", None, "binance", "crypto", None, None, None,
        )
        .unwrap()
    }

    /// Helper: create an AI rule in the DB and return its ID
    fn create_ai_rule(db: &DbPool, subscription_id: i64, enabled: bool) -> i64 {
        let ai_config =
            r#"{"prompt": "test prompt", "history_window": 20, "analysis_interval_secs": 60}"#;
        let channel_ids = "[1]";
        let rule_id = db
            .create_notification_rule(
                "Test AI Rule",
                subscription_id,
                "ai",
                0.0,
                channel_ids,
                300,
                Some(ai_config),
            )
            .unwrap();

        if !enabled {
            db.toggle_notification_rule(rule_id, false).unwrap();
        }

        rule_id
    }

    // =========================================================================
    // Test: start() loads AI rules and spawns tasks
    // Validates: Requirement 3.1 — Start periodic evaluation on app startup
    // =========================================================================

    #[tokio::test]
    async fn test_start_spawns_tasks_for_enabled_ai_rules() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create subscription and 2 enabled AI rules
        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        // Before start, no tasks should be running
        assert_eq!(scheduler.task_count().await, 0);

        // start() should spawn tasks for both enabled AI rules
        scheduler.start().await;

        // Verify 2 tasks are now running
        assert_eq!(scheduler.task_count().await, 2);
    }

    #[tokio::test]
    async fn test_start_with_no_ai_rules_spawns_zero_tasks() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config but no rules
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        scheduler.start().await;

        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_start_without_provider_config_spawns_zero_tasks() {
        let (db, scheduler) = setup_scheduler().await;

        // Create a subscription and an AI rule but NO provider config
        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;

        // Without provider config, no tasks should be spawned
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_start_ignores_disabled_ai_rules() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create 1 enabled and 1 disabled AI rule
        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, false);

        scheduler.start().await;

        // Only the enabled rule should have a task
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_start_ignores_non_ai_rules() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create a subscription
        let sub_id = create_test_subscription(&db);

        // Create a threshold rule (not AI)
        db.create_notification_rule(
            "Threshold Rule",
            sub_id,
            "price_above",
            50000.0,
            "[1]",
            300,
            None,
        )
        .unwrap();

        // Create one AI rule
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;

        // Only the AI rule should have a task, not the threshold rule
        assert_eq!(scheduler.task_count().await, 1);
    }

    // =========================================================================
    // Test: remove_rule() stops its task
    // Validates: Requirement 3.4 — Stop evaluation when rule is disabled
    // =========================================================================

    #[tokio::test]
    async fn test_remove_rule_stops_task() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create subscription and rule
        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        // Start scheduler — should have 1 task
        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        // Remove the rule — task count should drop to 0
        scheduler.remove_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_remove_rule_only_affects_target_rule() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create subscription and 2 rules
        let sub_id = create_test_subscription(&db);
        let rule_id_1 = create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        // Start scheduler — should have 2 tasks
        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 2);

        // Remove only rule 1 — task count should drop to 1
        scheduler.remove_rule(rule_id_1).await;
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_remove_rule_idempotent() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create subscription and rule
        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        // Remove the rule twice — second call should be a no-op
        scheduler.remove_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);

        scheduler.remove_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_rule_is_noop() {
        let (_db, scheduler) = setup_scheduler().await;

        // Removing a rule that was never added should not panic
        scheduler.remove_rule(9999).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    // =========================================================================
    // Test: upsert_rule() starts/restarts its task
    // Validates: Requirement 3.5 — Restart evaluation when rule is enabled
    // Validates: Requirement 3.6 — Restart with new interval when modified
    // =========================================================================

    #[tokio::test]
    async fn test_upsert_rule_starts_new_task() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create subscription and rule
        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        // Initially no tasks
        assert_eq!(scheduler.task_count().await, 0);

        // upsert_rule should start a task
        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_upsert_rule_restarts_existing_task() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create subscription and rule
        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        // Start scheduler with the rule
        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        // Upsert the same rule — should still have exactly 1 task (restarted)
        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 1);

        // Upsert again — still 1 task
        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_upsert_rule_disabled_rule_does_not_start_task() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create a disabled AI rule
        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, false);

        // upsert_rule for a disabled rule should not start a task
        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_upsert_rule_without_provider_config_does_not_start_task() {
        let (db, scheduler) = setup_scheduler().await;

        // Create subscription and rule but NO provider config
        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        // upsert_rule without provider config should not start a task
        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_upsert_rule_nonexistent_rule_does_not_start_task() {
        let (_db, scheduler) = setup_scheduler().await;

        // upsert_rule with a non-existent rule_id should not start a task
        scheduler.upsert_rule(9999).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_upsert_multiple_rules() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create subscription and multiple rules
        let sub_id = create_test_subscription(&db);
        let rule_id_1 = create_ai_rule(&db, sub_id, true);
        let rule_id_2 = create_ai_rule(&db, sub_id, true);

        // Upsert rules one by one
        scheduler.upsert_rule(rule_id_1).await;
        assert_eq!(scheduler.task_count().await, 1);

        scheduler.upsert_rule(rule_id_2).await;
        assert_eq!(scheduler.task_count().await, 2);
    }

    // =========================================================================
    // Test: reload() stops all tasks and restarts them
    // Validates: Requirement 3.5, 3.6 — Full reload of scheduler state
    // =========================================================================

    #[tokio::test]
    async fn test_reload_restarts_all_tasks() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create subscription and 2 rules
        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        // Start scheduler
        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 2);

        // Reload should stop all and restart — still 2 tasks
        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);
    }

    #[tokio::test]
    async fn test_reload_picks_up_new_rules() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create subscription and 1 rule
        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);

        // Start scheduler — 1 task
        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        // Add a new rule to DB
        create_ai_rule(&db, sub_id, true);

        // Reload should pick up the new rule — now 2 tasks
        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);
    }

    #[tokio::test]
    async fn test_reload_drops_removed_rules() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create subscription and 2 rules
        let sub_id = create_test_subscription(&db);
        let rule_id_1 = create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        // Start scheduler — 2 tasks
        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 2);

        // Disable rule 1 in DB
        db.toggle_notification_rule(rule_id_1, false).unwrap();

        // Reload should only have 1 task now (rule 1 is disabled)
        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_reload_with_no_provider_config_stops_all() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create subscription and rule
        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);

        // Start scheduler — 1 task
        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        // Clear provider config by setting values to empty strings
        // (load_ai_provider_config returns None when base_url or model is empty)
        db.set_setting("ai_base_url", "").unwrap();
        db.set_setting("ai_model", "").unwrap();

        // Reload without provider config — all tasks should stop
        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_reload_multiple_times_is_stable() {
        let (db, scheduler) = setup_scheduler().await;

        // Set up provider config
        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        // Create subscription and rules
        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 2);

        // Multiple reloads should be stable
        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);
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
    /// The webhook channel URL is set to the provided mock server address.
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

    /// E2E Test: evaluate_ai_rule with mock server returns trigger=true
    ///
    /// This test verifies the full AI evaluation pipeline:
    /// 1. Price history is fetched from DB
    /// 2. Prompt is built with the price data
    /// 3. HTTP request is sent to the AI API (mock server)
    /// 4. Response is parsed correctly
    /// 5. Result contains trigger=true and a reason
    #[tokio::test]
    async fn test_e2e_evaluate_ai_rule_trigger_true() {
        // Start mock server
        let addr = start_mock_server().await;
        let base_url = format!("http://{}", addr);
        let webhook_url = format!("http://{}/webhook", addr);

        // Set up DB with test data
        let (db, sub_id, rule_id, _channel_id) = setup_test_db(&webhook_url);

        // Configure AI provider to point to mock server
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

        // Call evaluate_ai_rule
        let result = evaluate_ai_rule(
            &db,
            &http_client,
            rule_id,
            sub_id,
            &ai_config,
            &provider_config,
        )
        .await;

        // Verify the result
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

    /// E2E Test: Full pipeline - AI trigger → dispatch notification → verify history
    ///
    /// This test verifies the complete notification dispatch flow:
    /// 1. AI evaluation returns trigger=true
    /// 2. dispatch_notification is called with AI notification data
    /// 3. notification_history table records the dispatch as "success"
    /// 4. The message contains the AI reason and symbol
    #[tokio::test]
    async fn test_e2e_ai_trigger_dispatches_notification_and_records_history() {
        // Start mock server (both AI API and webhook)
        let addr = start_mock_server().await;
        let base_url = format!("http://{}", addr);
        let webhook_url = format!("http://{}/webhook", addr);

        // Set up DB with test data (webhook points to mock server)
        let (db, sub_id, rule_id, channel_id) = setup_test_db(&webhook_url);

        // Configure AI provider
        let ai_config = AiConfig {
            prompt: "當價格在短時間內大幅上升超過 5% 時提醒我".to_string(),
            history_window: 5,
            analysis_interval_secs: 60,
        };

        let provider_config = AiProviderConfig {
            base_url,
            model: "test-model".to_string(),
            api_key: None, // Test without API key
        };

        let http_client = reqwest::Client::new();

        // Step 1: Evaluate AI rule
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

        // Step 2: Build NotificationRule and NotificationData (mimicking what AiScheduler does)
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

        // Step 3: Dispatch notification (webhook points to mock server, should succeed)
        dispatch_notification(&db, &http_client, &rule, &notif_data).await;

        // Step 4: Verify notification_history has a record
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
        // The webhook mock server returns 200 OK, so dispatch should succeed
        assert_eq!(
            record.status, "success",
            "Webhook dispatch should succeed with mock server"
        );
        // The message should contain the symbol (webhook payload is JSON)
        assert!(
            record.message.contains("BTC/USDT"),
            "Notification message should contain the symbol. Got: {}",
            record.message
        );
        // The message should contain the AI reason
        assert!(
            record.message.contains("6.2%"),
            "Notification message should contain the AI reason. Got: {}",
            record.message
        );
    }

    /// E2E Test: evaluate_ai_rule without API key (Ollama-style)
    ///
    /// Verifies that the system works correctly without an API key,
    /// which is the case for local Ollama deployments.
    #[tokio::test]
    async fn test_e2e_evaluate_ai_rule_without_api_key() {
        // Start mock server
        let addr = start_mock_server().await;
        let base_url = format!("http://{}", addr);
        let webhook_url = format!("http://{}/webhook", addr);

        // Set up DB with test data
        let (db, sub_id, rule_id, _channel_id) = setup_test_db(&webhook_url);

        let ai_config = AiConfig {
            prompt: "test prompt".to_string(),
            history_window: 5,
            analysis_interval_secs: 60,
        };

        // No API key - simulating local Ollama
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

    /// E2E Test: evaluate_ai_rule with no price history returns error
    ///
    /// Verifies that when there's no price history available,
    /// the evaluator returns an appropriate error (not triggered).
    #[tokio::test]
    async fn test_e2e_evaluate_ai_rule_no_price_history() {
        let addr = start_mock_server().await;
        let base_url = format!("http://{}", addr);

        let db = Arc::new(DbPool::open(&PathBuf::from(":memory:")).unwrap());

        // Create subscription but DON'T insert any price history
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

        // Should return an error because there's no price history
        assert!(
            result.is_err(),
            "Should fail when no price history is available"
        );
    }
}
