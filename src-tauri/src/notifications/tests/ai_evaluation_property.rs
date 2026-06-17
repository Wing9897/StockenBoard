//! AI evaluator and data model property-based tests (Properties 1-10)

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
                    sample_step: 1,
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
                None,
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
                sample_step: 1,
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
                sample_step: 1,
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
                sample_step: 1,
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
                disable_thinking: true,
                max_context_tokens: None,
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
                disable_thinking: true,
                max_context_tokens: None,
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
                api_key: None, disable_thinking: true, max_context_tokens: None, };
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
                disable_thinking: true,
                max_context_tokens: None,
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
                disable_thinking: true,
                max_context_tokens: None,
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
