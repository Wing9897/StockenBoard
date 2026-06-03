//! Live integration test against local Ollama
//!
//! Tests the AI evaluation pipeline by calling Ollama directly.
//! Requires a running Ollama instance at localhost:11434 with llama3.1:8b.
//!
//! Run with: cargo test --test ai_ollama_live_test -- --nocapture

use serde_json::json;

const OLLAMA_BASE_URL: &str = "http://localhost:11434/v1";
const MODEL: &str = "llama3.1:8b";

/// Check if Ollama is available
async fn ollama_available() -> bool {
    reqwest::Client::new()
        .get("http://localhost:11434/api/tags")
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .is_ok()
}

/// Call the AI API and return the parsed trigger/reason
async fn call_ai(prompt: &str, price_data: &str) -> (bool, String) {
    let system_msg = "你是一個金融市場分析助手。根據提供的價格歷史數據和使用者的觸發條件，判斷是否應該觸發通知。\n\n你必須以 JSON 格式回應：{\"trigger\": true/false, \"reason\": \"簡短說明理由\"}\n\n只回傳 JSON，不要包含其他文字。";

    let user_msg = format!(
        "觸發條件：{}\n\n最近價格紀錄：\n| 時間 | 價格 | 漲跌幅(%) | 成交量 |\n|------|------|-----------|--------|\n{}\n\n請判斷是否滿足觸發條件。",
        prompt, price_data
    );

    let body = json!({
        "model": MODEL,
        "messages": [
            {"role": "system", "content": system_msg},
            {"role": "user", "content": user_msg}
        ],
        "temperature": 0.1,
        "max_tokens": 200
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/chat/completions", OLLAMA_BASE_URL))
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .expect("Failed to send request to Ollama");

    assert!(resp.status().is_success(), "Ollama returned error: {}", resp.status());

    let resp_json: serde_json::Value = resp.json().await.expect("Failed to parse response");
    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .expect("Missing content in response");

    // Parse the AI response (handle potential markdown wrapping)
    let trimmed = content.trim();
    let json_str = if trimmed.starts_with("```") {
        // Extract from code block
        let start = trimmed.find('{').unwrap_or(0);
        let end = trimmed.rfind('}').unwrap_or(trimmed.len() - 1) + 1;
        &trimmed[start..end]
    } else {
        trimmed
    };

    let parsed: serde_json::Value = serde_json::from_str(json_str)
        .unwrap_or_else(|e| panic!("Failed to parse AI JSON response: {}\nRaw: {}", e, content));

    let trigger = parsed["trigger"].as_bool().expect("Missing trigger field");
    let reason = parsed["reason"].as_str().unwrap_or("").to_string();

    (trigger, reason)
}

/// Test: Clear uptrend (6.2% rise) should trigger
#[tokio::test]
async fn test_ollama_trigger_on_clear_uptrend() {
    if !ollama_available().await {
        eprintln!("⚠️  Ollama not available at localhost:11434, skipping");
        return;
    }

    let price_data = "| 2024-01-15 10:30 | 68500.00 | +6.2% | 1234.5 |\n\
                      | 2024-01-15 10:25 | 67800.00 | +4.1% | 1100.0 |\n\
                      | 2024-01-15 10:20 | 66500.00 | +2.5% | 980.0 |\n\
                      | 2024-01-15 10:15 | 65200.00 | +1.0% | 850.0 |\n\
                      | 2024-01-15 10:10 | 64500.00 | -0.5% | 750.0 |";

    eprintln!("🔄 Testing: clear uptrend (+6.2%), expecting trigger=true...");
    let (trigger, reason) = call_ai("Alert me when price rises more than 5% in a short time", price_data).await;
    eprintln!("✅ Result: trigger={}, reason={}", trigger, reason);

    assert!(trigger, "Expected trigger=true for 6.2% rise. AI said: {}", reason);
}

/// Test: Stable prices should NOT trigger
#[tokio::test]
async fn test_ollama_no_trigger_on_stable_prices() {
    if !ollama_available().await {
        eprintln!("⚠️  Ollama not available at localhost:11434, skipping");
        return;
    }

    let price_data = "| 2024-01-15 10:30 | 65050.00 | +0.08% | 800.0 |\n\
                      | 2024-01-15 10:25 | 65020.00 | +0.03% | 790.0 |\n\
                      | 2024-01-15 10:20 | 64980.00 | -0.06% | 810.0 |\n\
                      | 2024-01-15 10:15 | 65010.00 | +0.05% | 795.0 |\n\
                      | 2024-01-15 10:10 | 65000.00 | +0.00% | 800.0 |";

    eprintln!("🔄 Testing: stable prices (<0.1% change), expecting trigger=false...");
    let (trigger, reason) = call_ai("Alert me when price rises more than 5% in a short time", price_data).await;
    eprintln!("✅ Result: trigger={}, reason={}", trigger, reason);

    assert!(!trigger, "Expected trigger=false for stable prices. AI said: {}", reason);
}

/// Test: AI response parsing handles various Ollama output formats
#[test]
fn test_parse_various_ollama_formats() {
    // Plain JSON
    let raw1 = r#"{"trigger": true, "reason": "Price rose significantly"}"#;
    let v1: serde_json::Value = serde_json::from_str(raw1).unwrap();
    assert!(v1["trigger"].as_bool().unwrap());

    // JSON with newlines (common Ollama format)
    let raw2 = "{\n  \"trigger\": false,\n  \"reason\": \"No significant change detected\"\n}";
    let v2: serde_json::Value = serde_json::from_str(raw2).unwrap();
    assert!(!v2["trigger"].as_bool().unwrap());

    // JSON in markdown code block
    let raw3 = "```json\n{\"trigger\": true, \"reason\": \"test\"}\n```";
    let start = raw3.find('{').unwrap();
    let end = raw3.rfind('}').unwrap() + 1;
    let v3: serde_json::Value = serde_json::from_str(&raw3[start..end]).unwrap();
    assert!(v3["trigger"].as_bool().unwrap());
}
