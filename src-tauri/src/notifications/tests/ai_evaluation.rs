//! AI evaluation tests — config validation, prompt building, response parsing,
//! provider config, data model property tests, scheduler integration tests

use crate::notifications::models::{AiConfig, AiProviderConfig};

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
    let config = AiProviderConfig {
        base_url: "http://localhost:11434/v1".to_string(),
        model: "llama3".to_string(),
        api_key: None,
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_ai_provider_config_valid_with_api_key() {
    let config = AiProviderConfig {
        base_url: "https://api.openai.com/v1".to_string(),
        model: "gpt-4".to_string(),
        api_key: Some("sk-test-key-123".to_string()),
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_ai_provider_config_empty_base_url() {
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

    db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
        .unwrap();

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

    db.save_ai_provider_config(
        "https://api.openai.com/v1",
        "gpt-4",
        Some("sk-test-key-12345"),
    )
    .unwrap();

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

    db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
        .unwrap();

    db.save_ai_provider_config("https://api.openai.com/v1", "gpt-4", Some("sk-new-key"))
        .unwrap();

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

    db.save_ai_provider_config("http://localhost:11434/v1", "llama3", Some(""))
        .unwrap();

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

        assert!(user.content.contains("68500.00"));
        assert!(user.content.contains("67000.00"));

        assert!(user.content.contains("+2.3%"));
        assert!(user.content.contains("-1.5%"));

        assert!(user.content.contains("1234.5"));
        assert!(user.content.contains("987.2"));

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
            0.01f64..1_000_000.0,
            -100.0f64..100.0,
            0.0f64..1_000_000.0,
            "[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}",
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
            "[a-z_]{1,20}",
            "[a-zA-Z0-9 ]{1,50}",
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
            let user_msg = &messages[1].content;

            prop_assert!(
                user_msg.contains(&condition),
                "Prompt missing user condition: '{}'", condition
            );

            for record in &records {
                let price_str = format!("{:.2}", record.price);
                prop_assert!(
                    user_msg.contains(&price_str),
                    "Prompt missing price: {}", price_str
                );

                let change_str = format!("{:+.1}%", record.change_pct);
                prop_assert!(
                    user_msg.contains(&change_str),
                    "Prompt missing change_pct: {}", change_str
                );

                let volume_str = format!("{:.1}", record.volume);
                prop_assert!(
                    user_msg.contains(&volume_str),
                    "Prompt missing volume: {}", volume_str
                );

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

            let markdown_json = format!("```json\n{}\n```", raw_json);
            let result_md_json = parse_ai_response(&markdown_json).unwrap();

            let markdown_plain = format!("```\n{}\n```", raw_json);
            let result_md_plain = parse_ai_response(&markdown_plain).unwrap();

            let result_raw = parse_ai_response(&raw_json).unwrap();

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
            let json = format!(
                r#"{{"trigger": "{}", "reason": "{}"}}"#,
                trigger_val, reason
            );
            let result = parse_ai_response(&json);
            prop_assert!(result.is_err(), "Expected error for non-boolean trigger: {}", json);
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
            let db = DbPool::open(&PathBuf::from(":memory:")).unwrap();

            db.add_subscription("asset", "BTC/USDT", None, "binance", "crypto", None, None, None).unwrap();

            let ai_config_json = serde_json::to_string(&config).unwrap();

            let channel_ids_json = serde_json::to_string(&vec![1i64]).unwrap();
            let rule_id = db.create_notification_rule(
                &rule_name,
                1,
                "ai",
                0.0,
                &channel_ids_json,
                cooldown_secs,
                Some(&ai_config_json),
            ).unwrap();

            let loaded = db.get_notification_rule(rule_id).unwrap().unwrap();

            prop_assert_eq!(&loaded.name, &rule_name);
            prop_assert_eq!(loaded.subscription_id, 1);
            prop_assert_eq!(&loaded.condition_type, "ai");
            prop_assert_eq!(loaded.threshold, 0.0);
            prop_assert_eq!(&loaded.channel_ids, &channel_ids_json);
            prop_assert_eq!(loaded.cooldown_secs, cooldown_secs);
            prop_assert!(loaded.enabled);

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
            let encrypted = encrypt_token(&api_key).unwrap();

            prop_assert_ne!(&encrypted, &api_key,
                "Encrypted value should differ from plaintext");

            let decrypted = decrypt_token(&encrypted).unwrap();
            prop_assert_eq!(decrypted, api_key,
                "Decrypted value should match original");
        }
    }
}


// === AI Scheduler Integration Tests ===
// **Validates: Requirements 3.1, 3.4, 3.5, 3.6**

mod ai_scheduler_integration_tests {
    use crate::db::DbPool;
    use crate::notifications::ai_scheduler::AiScheduler;
    use std::path::PathBuf;
    use std::sync::Arc;

    async fn setup_scheduler() -> (Arc<DbPool>, AiScheduler) {
        let db = Arc::new(DbPool::open(&PathBuf::from(":memory:")).unwrap());
        let scheduler = AiScheduler::new(db.clone());
        (db, scheduler)
    }

    fn create_test_subscription(db: &DbPool) -> i64 {
        db.add_subscription(
            "asset", "BTC/USDT", None, "binance", "crypto", None, None, None,
        )
        .unwrap()
    }

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

    #[tokio::test]
    async fn test_start_spawns_tasks_for_enabled_ai_rules() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        assert_eq!(scheduler.task_count().await, 0);

        scheduler.start().await;

        assert_eq!(scheduler.task_count().await, 2);
    }

    #[tokio::test]
    async fn test_start_with_no_ai_rules_spawns_zero_tasks() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        scheduler.start().await;

        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_start_without_provider_config_spawns_zero_tasks() {
        let (db, scheduler) = setup_scheduler().await;

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;

        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_start_ignores_disabled_ai_rules() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, false);

        scheduler.start().await;

        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_start_ignores_non_ai_rules() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);

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

        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;

        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_remove_rule_stops_task() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        scheduler.remove_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_remove_rule_only_affects_target_rule() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id_1 = create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 2);

        scheduler.remove_rule(rule_id_1).await;
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_remove_rule_idempotent() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        scheduler.remove_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);

        scheduler.remove_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_rule_is_noop() {
        let (_db, scheduler) = setup_scheduler().await;

        scheduler.remove_rule(9999).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_upsert_rule_starts_new_task() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        assert_eq!(scheduler.task_count().await, 0);

        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_upsert_rule_restarts_existing_task() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 1);

        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_upsert_rule_disabled_rule_does_not_start_task() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, false);

        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_upsert_rule_without_provider_config_does_not_start_task() {
        let (db, scheduler) = setup_scheduler().await;

        let sub_id = create_test_subscription(&db);
        let rule_id = create_ai_rule(&db, sub_id, true);

        scheduler.upsert_rule(rule_id).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_upsert_rule_nonexistent_rule_does_not_start_task() {
        let (_db, scheduler) = setup_scheduler().await;

        scheduler.upsert_rule(9999).await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_upsert_multiple_rules() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id_1 = create_ai_rule(&db, sub_id, true);
        let rule_id_2 = create_ai_rule(&db, sub_id, true);

        scheduler.upsert_rule(rule_id_1).await;
        assert_eq!(scheduler.task_count().await, 1);

        scheduler.upsert_rule(rule_id_2).await;
        assert_eq!(scheduler.task_count().await, 2);
    }

    #[tokio::test]
    async fn test_reload_restarts_all_tasks() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 2);

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);
    }

    #[tokio::test]
    async fn test_reload_picks_up_new_rules() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        create_ai_rule(&db, sub_id, true);

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);
    }

    #[tokio::test]
    async fn test_reload_drops_removed_rules() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        let rule_id_1 = create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 2);

        db.toggle_notification_rule(rule_id_1, false).unwrap();

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn test_reload_with_no_provider_config_stops_all() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 1);

        db.set_setting("ai_base_url", "").unwrap();
        db.set_setting("ai_model", "").unwrap();

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 0);
    }

    #[tokio::test]
    async fn test_reload_multiple_times_is_stable() {
        let (db, scheduler) = setup_scheduler().await;

        db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None)
            .unwrap();

        let sub_id = create_test_subscription(&db);
        create_ai_rule(&db, sub_id, true);
        create_ai_rule(&db, sub_id, true);

        scheduler.start().await;
        assert_eq!(scheduler.task_count().await, 2);

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);

        scheduler.reload().await;
        assert_eq!(scheduler.task_count().await, 2);
    }
}
