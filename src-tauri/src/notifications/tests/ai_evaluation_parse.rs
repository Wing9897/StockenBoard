//! parse_ai_response unit tests

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
