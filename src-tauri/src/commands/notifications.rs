use crate::core_state::CoreState;
use std::sync::Arc;

// ── Global Cooldown Commands ────────────────────────────────────

#[tauri::command]
pub async fn get_notification_global_cooldown(
    state: tauri::State<'_, Arc<CoreState>>,
) -> Result<u64, String> {
    let val = state
        .db
        .get_setting("notification_global_cooldown")?
        .unwrap_or_else(|| "30".into());
    val.parse::<u64>()
        .map_err(|e| format!("Invalid cooldown value: {}", e))
}

#[tauri::command]
pub async fn set_notification_global_cooldown(
    state: tauri::State<'_, Arc<CoreState>>,
    secs: u64,
) -> Result<(), String> {
    state
        .db
        .set_setting("notification_global_cooldown", &secs.to_string())?;
    state.global_cooldown.set_cooldown(secs);
    Ok(())
}

// ── Notification Rule Commands ──────────────────────────────────

#[tauri::command]
pub async fn create_notification_rule(
    state: tauri::State<'_, Arc<CoreState>>,
    rule: crate::notifications::models::CreateRuleRequest,
) -> Result<i64, String> {
    // Validate AI config when condition_type is "ai"
    let threshold = if rule.condition_type == "ai" {
        // ai_config is required for AI rules
        let ai_config = rule
            .ai_config
            .as_ref()
            .ok_or_else(|| "ai_config is required when condition_type is \"ai\"".to_string())?;
        // Validate ai_config fields
        ai_config.validate()?;
        // AI rules use threshold 0.0
        0.0
    } else {
        rule.threshold
    };

    let channel_ids_json = serde_json::to_string(&rule.channel_ids)
        .map_err(|e| format!("Failed to serialize channel_ids: {}", e))?;
    let cooldown = rule.cooldown_secs.unwrap_or(300) as i64;
    let ai_config_json = rule
        .ai_config
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| format!("Failed to serialize ai_config: {}", e))?;
    let id = state.db.create_notification_rule(
        &rule.name,
        rule.subscription_id,
        &rule.condition_type,
        threshold,
        &channel_ids_json,
        cooldown,
        ai_config_json.as_deref(),
    )?;
    state.notification_engine.reload_rules().await;
    // Notify AI scheduler to pick up the new rule if it's an AI rule
    if rule.condition_type == "ai" {
        state.ai_scheduler.upsert_rule(id).await;
    }
    Ok(id)
}

#[tauri::command]
pub async fn list_notification_rules(
    state: tauri::State<'_, Arc<CoreState>>,
) -> Result<Vec<crate::db::NotificationRuleRow>, String> {
    state.db.list_notification_rules()
}

#[tauri::command]
pub async fn update_notification_rule(
    state: tauri::State<'_, Arc<CoreState>>,
    id: i64,
    rule: crate::notifications::models::UpdateRuleRequest,
) -> Result<(), String> {
    // Validate AI config if provided
    if let Some(Some(ref ai_config)) = rule.ai_config {
        ai_config.validate()?;
    }

    // If switching to AI type, ensure ai_config is provided
    if let Some(ref ct) = rule.condition_type {
        if ct == "ai" {
            match &rule.ai_config {
                Some(Some(_)) => {} // ai_config provided, OK
                _ => return Err("ai_config is required when condition_type is \"ai\"".to_string()),
            }
        }
    }

    let channel_ids_json = rule
        .channel_ids
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| format!("Failed to serialize channel_ids: {}", e))?;

    // ai_config: Option<Option<AiConfig>> -> Option<Option<String>>
    // Some(Some(cfg)) => set ai_config to JSON string
    // Some(None) => set ai_config to NULL
    // None => don't update ai_config
    let ai_config_json: Option<Option<String>> = match &rule.ai_config {
        Some(Some(cfg)) => {
            let json =
                serde_json::to_string(cfg).map_err(|e| format!("Failed to serialize ai_config: {}", e))?;
            Some(Some(json))
        }
        Some(None) => Some(None),
        None => None,
    };

    // If switching to AI type, set threshold to 0.0
    let threshold = if rule.condition_type.as_deref() == Some("ai") {
        Some(0.0)
    } else {
        rule.threshold
    };

    state.db.update_notification_rule(
        id,
        rule.name.as_deref(),
        rule.condition_type.as_deref(),
        threshold,
        channel_ids_json.as_deref(),
        rule.cooldown_secs.map(|s| s as i64),
        ai_config_json.as_ref().map(|opt| opt.as_deref()),
    )?;
    state.notification_engine.reload_rules().await;

    // Notify AI scheduler about the update
    // If switching to AI or updating AI config, upsert the rule
    // If switching away from AI (ai_config set to None), remove the rule
    match &rule.ai_config {
        Some(None) => {
            // Clearing ai_config - remove from scheduler
            state.ai_scheduler.remove_rule(id).await;
        }
        Some(Some(_)) => {
            // AI config updated or switching to AI - upsert
            state.ai_scheduler.upsert_rule(id).await;
        }
        None => {
            // ai_config not being updated, but condition_type might have changed
            if let Some(ref ct) = rule.condition_type {
                if ct != "ai" {
                    // Switching away from AI type
                    state.ai_scheduler.remove_rule(id).await;
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_notification_rule(
    state: tauri::State<'_, Arc<CoreState>>,
    id: i64,
) -> Result<(), String> {
    state.db.delete_notification_rule(id)?;
    state.notification_engine.reload_rules().await;
    // Notify AI scheduler to stop any running task for this rule
    state.ai_scheduler.remove_rule(id).await;
    Ok(())
}

#[tauri::command]
pub async fn toggle_notification_rule(
    state: tauri::State<'_, Arc<CoreState>>,
    id: i64,
    enabled: bool,
) -> Result<(), String> {
    state.db.toggle_notification_rule(id, enabled)?;
    state.notification_engine.reload_rules().await;
    // Notify AI scheduler about the toggle
    if enabled {
        // Re-enable: upsert will start the task if it's an AI rule
        state.ai_scheduler.upsert_rule(id).await;
    } else {
        // Disable: remove the task from scheduler
        state.ai_scheduler.remove_rule(id).await;
    }
    Ok(())
}

// ── Notification Channel Commands ───────────────────────────────

#[tauri::command]
pub async fn save_notification_channel(
    state: tauri::State<'_, Arc<CoreState>>,
    channel: crate::notifications::models::SaveChannelRequest,
) -> Result<i64, String> {
    // Validate config based on channel_type
    match channel.channel_type.as_str() {
        "telegram" => {
            let config: crate::notifications::models::TelegramConfig =
                serde_json::from_str(&channel.config)
                    .map_err(|e| format!("Invalid Telegram config format: {}", e))?;
            if config.bot_token.is_empty() || config.chat_id.is_empty() {
                return Err("Bot Token and Chat ID must not be empty".to_string());
            }
            // Encrypt bot_token before storing
            let encrypted_token = crate::notifications::crypto::encrypt_token(&config.bot_token)?;
            let stored_config = serde_json::json!({
                "bot_token": encrypted_token,
                "chat_id": config.chat_id,
            });
            state.db.create_notification_channel(
                &channel.channel_type,
                &channel.name,
                &stored_config.to_string(),
            )
        }
        "webhook" => {
            let config: crate::notifications::models::WebhookConfig =
                serde_json::from_str(&channel.config)
                    .map_err(|e| format!("Invalid Webhook config format: {}", e))?;
            if config.url.is_empty() {
                return Err("Webhook URL must not be empty".to_string());
            }
            state.db.create_notification_channel(
                &channel.channel_type,
                &channel.name,
                &channel.config,
            )
        }
        _ => Err(format!("Unsupported channel type: {}", channel.channel_type)),
    }
}

#[tauri::command]
pub async fn list_notification_channels(
    state: tauri::State<'_, Arc<CoreState>>,
) -> Result<Vec<crate::db::NotificationChannelRow>, String> {
    state.db.list_notification_channels()
}

#[tauri::command]
pub async fn delete_notification_channel(
    state: tauri::State<'_, Arc<CoreState>>,
    id: i64,
) -> Result<(), String> {
    state.db.delete_notification_channel(id)
}

#[tauri::command]
pub async fn test_notification_channel(
    state: tauri::State<'_, Arc<CoreState>>,
    id: i64,
) -> Result<(), String> {
    let channels = state.db.list_notification_channels()?;
    let channel = channels
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("Channel {} not found", id))?;

    let client = reqwest::Client::new();

    match channel.channel_type.as_str() {
        "telegram" => {
            let stored_config: serde_json::Value = serde_json::from_str(&channel.config)
                .map_err(|e| format!("Failed to parse config: {}", e))?;
            let encrypted_token = stored_config["bot_token"]
                .as_str()
                .ok_or("Missing bot_token")?;
            let chat_id = stored_config["chat_id"].as_str().ok_or("Missing chat_id")?;
            let bot_token = crate::notifications::crypto::decrypt_token(encrypted_token)?;
            let config = crate::notifications::models::TelegramConfig {
                bot_token,
                chat_id: chat_id.to_string(),
            };
            let test_message =
                "🔔 StockenBoard Test Notification\n\nThis is a test message to confirm the Telegram channel is configured correctly.";
            crate::notifications::telegram::send_telegram(&client, &config, test_message).await
        }
        "webhook" => {
            let config: crate::notifications::models::WebhookConfig =
                serde_json::from_str(&channel.config)
                    .map_err(|e| format!("Failed to parse config: {}", e))?;
            let test_data = crate::notifications::models::NotificationData {
                symbol: "TEST/USD".to_string(),
                provider: "test".to_string(),
                price: 100.0,
                condition_type: crate::notifications::models::ConditionType::PriceAbove,
                threshold: 99.0,
                rule_name: "Test rule".to_string(),
                triggered_at: chrono::Utc::now(),
            };
            crate::notifications::webhook::send_webhook(&client, &config, &test_data).await
        }
        _ => Err(format!("Unsupported channel type: {}", channel.channel_type)),
    }
}

// ── Notification History Commands ───────────────────────────────

#[tauri::command]
pub async fn get_notification_history(
    state: tauri::State<'_, Arc<CoreState>>,
    rule_id: Option<i64>,
    from: Option<i64>,
    to: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<crate::db::NotificationHistoryRow>, String> {
    state
        .db
        .query_notification_history(rule_id, from, to, limit)
}

// ── AI Provider Config Commands ─────────────────────────────────

#[tauri::command]
pub async fn save_ai_provider_config(
    state: tauri::State<'_, Arc<CoreState>>,
    base_url: String,
    model: String,
    api_key: Option<String>,
) -> Result<(), String> {
    state
        .db
        .save_ai_provider_config(&base_url, &model, api_key.as_deref())?;
    state.ai_scheduler.reload().await;
    Ok(())
}

#[tauri::command]
pub async fn get_ai_provider_config(
    state: tauri::State<'_, Arc<CoreState>>,
) -> Result<Option<crate::notifications::models::AiProviderConfigResponse>, String> {
    let config = state.db.load_ai_provider_config()?;
    Ok(
        config.map(|c| crate::notifications::models::AiProviderConfigResponse {
            base_url: c.base_url,
            model: c.model,
            has_api_key: c.api_key.is_some(),
        }),
    )
}

#[tauri::command]
pub async fn test_ai_connection(state: tauri::State<'_, Arc<CoreState>>) -> Result<String, String> {
    // 1. Load AI provider config from DB
    let config = state
        .db
        .load_ai_provider_config()?
        .ok_or_else(|| "AI provider not configured, please set base_url and model first".to_string())?;

    // 2. Build the test request URL
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    // 3. Build the request body
    let body = serde_json::json!({
        "model": config.model,
        "messages": [{"role": "user", "content": "Hello"}],
        "max_tokens": 5
    });

    // 4. Create HTTP client with 10s timeout and send request
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut request = client.post(&url).json(&body);

    // Include Authorization header if api_key is set
    if let Some(ref api_key) = config.api_key {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    // 5. Check response status
    let status = response.status();
    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return Err(format!(
            "AI API error (HTTP {}): {}",
            status.as_u16(),
            error_body
        ));
    }

    // 6. Parse response to extract model name
    let resp_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let model_name = resp_json
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or(&config.model);

    Ok(format!("Connection successful! Model: {}", model_name))
}

#[tauri::command]
pub async fn list_ai_models(base_url: String, api_key: Option<String>) -> Result<Vec<String>, String> {
    // Try Ollama-style /api/tags endpoint first, then OpenAI-style /models
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let trimmed_url = base_url.trim_end_matches('/');

    // Try Ollama native API: {base_url without /v1}/api/tags
    let ollama_base = trimmed_url.trim_end_matches("/v1");
    let ollama_url = format!("{}/api/tags", ollama_base);

    if let Ok(resp) = client.get(&ollama_url).send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(models) = json.get("models").and_then(|m| m.as_array()) {
                    let names: Vec<String> = models
                        .iter()
                        .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                        .collect();
                    if !names.is_empty() {
                        return Ok(names);
                    }
                }
            }
        }
    }

    // Try OpenAI-compatible /models endpoint
    let openai_url = format!("{}/models", trimmed_url);
    let mut req = client.get(&openai_url);
    if let Some(ref key) = api_key {
        if !key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
    }

    if let Ok(resp) = req.send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                    let names: Vec<String> = data
                        .iter()
                        .filter_map(|m| m.get("id").and_then(|n| n.as_str()).map(|s| s.to_string()))
                        .collect();
                    if !names.is_empty() {
                        return Ok(names);
                    }
                }
            }
        }
    }

    Err("Failed to list models, please verify URL is correct".to_string())
}
