//! AI config and provider config validation tests + DB persistence tests

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
        api_key: None, disable_thinking: true, max_context_tokens: None, };
    assert!(config.validate().is_ok());
}

#[test]
fn test_ai_provider_config_valid_with_api_key() {
    let config = AiProviderConfig {
        base_url: "https://api.openai.com/v1".to_string(),
        model: "gpt-4".to_string(),
        api_key: Some("sk-test-key-123".to_string()), disable_thinking: true, max_context_tokens: None, };
    assert!(config.validate().is_ok());
}

#[test]
fn test_ai_provider_config_empty_base_url() {
    let config = AiProviderConfig {
        base_url: "".to_string(),
        model: "llama3".to_string(),
        api_key: None, disable_thinking: true, max_context_tokens: None, };
    let err = config.validate().unwrap_err();
    assert!(err.contains("base_url must not be empty"));
}

#[test]
fn test_ai_provider_config_whitespace_base_url() {
    let config = AiProviderConfig {
        base_url: "   ".to_string(),
        model: "llama3".to_string(),
        api_key: None, disable_thinking: true, max_context_tokens: None, };
    let err = config.validate().unwrap_err();
    assert!(err.contains("base_url must not be empty"));
}

#[test]
fn test_ai_provider_config_empty_model() {
    let config = AiProviderConfig {
        base_url: "http://localhost:11434/v1".to_string(),
        model: "".to_string(),
        api_key: None, disable_thinking: true, max_context_tokens: None, };
    let err = config.validate().unwrap_err();
    assert!(err.contains("model must not be empty"));
}

#[test]
fn test_ai_provider_config_whitespace_model() {
    let config = AiProviderConfig {
        base_url: "http://localhost:11434/v1".to_string(),
        model: "  ".to_string(),
        api_key: None, disable_thinking: true, max_context_tokens: None, };
    let err = config.validate().unwrap_err();
    assert!(err.contains("model must not be empty"));
}

// === AI Provider Config DB 讀寫測試 ===

#[test]
fn test_save_and_load_ai_provider_config_without_api_key() {
    use crate::db::DbPool;
    use std::path::PathBuf;

    let db = DbPool::open(&PathBuf::from(":memory:")).unwrap();

    db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
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
        true,
        None,
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

    let result = db.save_ai_provider_config("", "llama3", None, false, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("base_url must not be empty"));
}

#[test]
fn test_save_ai_provider_config_validates_model() {
    use crate::db::DbPool;
    use std::path::PathBuf;

    let db = DbPool::open(&PathBuf::from(":memory:")).unwrap();

    let result = db.save_ai_provider_config("http://localhost:11434/v1", "", None, false, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("model must not be empty"));
}

#[test]
fn test_save_ai_provider_config_overwrites_existing() {
    use crate::db::DbPool;
    use std::path::PathBuf;

    let db = DbPool::open(&PathBuf::from(":memory:")).unwrap();

    db.save_ai_provider_config("http://localhost:11434/v1", "llama3", None, false, None)
        .unwrap();

    db.save_ai_provider_config("https://api.openai.com/v1", "gpt-4", Some("sk-new-key"), false, None)
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

    db.save_ai_provider_config("http://localhost:11434/v1", "llama3", Some(""), false, None)
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
    db.save_ai_provider_config("http://localhost:11434/v1", "llama3", Some(api_key), false, None)
        .unwrap();

    let raw_value = db.get_setting("ai_api_key").unwrap().unwrap();
    assert_ne!(raw_value, api_key);
    assert!(!raw_value.is_empty());
}
