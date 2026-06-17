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
    let cooldown = rule.cooldown_secs.unwrap_or(0) as i64;
    let ai_config_json = rule
        .ai_config
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| format!("Failed to serialize ai_config: {}", e))?;

    // For AI rules: use subscription_ids to set both subscription_ids column and subscription_id
    // For threshold rules: ignore subscription_ids, use subscription_id directly
    let (effective_subscription_id, subscription_ids_json) = if rule.condition_type == "ai" {
        if let Some(ref ids) = rule.subscription_ids {
            if !ids.is_empty() {
                // Set subscription_id to first element for backward compatibility
                let first_id = ids[0];
                let json = serde_json::to_string(ids)
                    .map_err(|e| format!("Failed to serialize subscription_ids: {}", e))?;
                (first_id, Some(json))
            } else {
                // Empty array: use the provided subscription_id as fallback
                (rule.subscription_id, None)
            }
        } else {
            // No subscription_ids provided: use subscription_id as-is (backward compat)
            (rule.subscription_id, None)
        }
    } else {
        // Threshold rules: ignore subscription_ids, use subscription_id directly
        (rule.subscription_id, None)
    };

    let id = state.db.create_notification_rule(
        &rule.name,
        effective_subscription_id,
        &rule.condition_type,
        threshold,
        &channel_ids_json,
        cooldown,
        ai_config_json.as_deref(),
        subscription_ids_json.as_deref(),
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

    // Handle subscription_ids for AI rules:
    // If subscription_ids is provided and non-empty, serialize to JSON and set subscription_id to first element
    // For threshold rules: ignore subscription_ids
    let is_ai_rule = rule.condition_type.as_deref() == Some("ai")
        || (rule.condition_type.is_none() && rule.ai_config.is_some() && rule.ai_config != Some(None));

    let (subscription_ids_param, subscription_id_param): (Option<Option<String>>, Option<i64>) =
        if is_ai_rule {
            if let Some(ref ids) = rule.subscription_ids {
                if !ids.is_empty() {
                    let json = serde_json::to_string(ids)
                        .map_err(|e| format!("Failed to serialize subscription_ids: {}", e))?;
                    // Backward compatibility: subscription_id = first element
                    (Some(Some(json)), Some(ids[0]))
                } else {
                    // Empty array provided: clear subscription_ids (set to NULL)
                    (Some(None), None)
                }
            } else {
                // subscription_ids not provided in update: don't change it
                (None, None)
            }
        } else {
            // Threshold rules: ignore subscription_ids
            (None, None)
        };

    state.db.update_notification_rule(
        id,
        rule.name.as_deref(),
        rule.condition_type.as_deref(),
        threshold,
        channel_ids_json.as_deref(),
        rule.cooldown_secs.map(|s| s as i64),
        ai_config_json.as_ref().map(|opt| opt.as_deref()),
        subscription_ids_param.as_ref().map(|opt| opt.as_deref()),
        subscription_id_param,
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
    // Prevent deletion of the built-in local and system channels
    let channels = state.db.list_notification_channels()?;
    if let Some(ch) = channels.iter().find(|c| c.id == id) {
        if ch.channel_type == "local" || ch.channel_type == "system" {
            return Err("Cannot delete the built-in notification channel".to_string());
        }
    }
    state.db.delete_notification_channel(id)
}

#[tauri::command]
pub async fn test_notification_channel(
    state: tauri::State<'_, Arc<CoreState>>,
    app: tauri::AppHandle,
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
        "local" => {
            // Emit a test notification event to the frontend
            use tauri::Emitter;
            let payload = crate::events::NotificationTriggeredPayload {
                rule_name: "Test Notification".to_string(),
                symbol: "TEST/USD".to_string(),
                provider: "test".to_string(),
                price: 0.0,
                condition_type: "price_above".to_string(),
                threshold: 0.0,
                triggered_at: chrono::Utc::now().timestamp(),
                is_ai: false,
                ai_reason: None,
            };
            app.emit("notification-triggered", &payload)
                .map_err(|e| format!("Failed to emit test notification: {}", e))?;
            Ok(())
        }
        "system" => {
            // Send a native OS notification
            use tauri_plugin_notification::NotificationExt;
            app.notification()
                .builder()
                .title("StockenBoard")
                .body("🔔 Test system notification — working correctly!")
                .show()
                .map_err(|e| format!("Failed to send system notification: {}", e))?;
            Ok(())
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
    disable_thinking: Option<bool>,
    max_context_tokens: Option<u32>,
) -> Result<(), String> {
    state
        .db
        .save_ai_provider_config(&base_url, &model, api_key.as_deref(), disable_thinking.unwrap_or(true), max_context_tokens)?;
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
            disable_thinking: c.disable_thinking,
            max_context_tokens: c.max_context_tokens,
        }),
    )
}

#[tauri::command]
pub async fn test_ai_connection(
    state: tauri::State<'_, Arc<CoreState>>,
    base_url: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
) -> Result<String, String> {
    // 1. Determine config: use provided values or fall back to DB
    let db_config = state.db.load_ai_provider_config()?;

    let effective_base_url = base_url
        .filter(|u| !u.is_empty())
        .or_else(|| db_config.as_ref().map(|c| c.base_url.clone()))
        .ok_or_else(|| "No base_url provided and none saved".to_string())?;

    let effective_model = model
        .filter(|m| !m.is_empty())
        .or_else(|| db_config.as_ref().map(|c| c.model.clone()))
        .ok_or_else(|| "No model provided and none saved".to_string())?;

    let effective_api_key = api_key
        .filter(|k| !k.is_empty())
        .or_else(|| db_config.and_then(|c| c.api_key));

    // 2. Build the test request URL
    let url = format!("{}/chat/completions", effective_base_url.trim_end_matches('/'));

    // 3. Build a test prompt that validates JSON output capability
    //    Use response_format to force JSON mode and reasoning_effort "none" to disable thinking
    let body = serde_json::json!({
        "model": effective_model,
        "messages": [
            {
                "role": "system",
                "content": "You are a JSON output validator. Respond ONLY with valid JSON."
            },
            {
                "role": "user",
                "content": "Respond with exactly this JSON: {\"trigger\": false, \"reason\": \"test\"}"
            }
        ],
        "max_tokens": 50,
        "response_format": {"type": "json_object"},
        "reasoning_effort": "none"
    });

    // 4. Create HTTP client with 15s timeout and send request
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut request = client.post(&url).json(&body);

    if let Some(ref key) = effective_api_key {
        request = request.header("Authorization", format!("Bearer {}", key));
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

    // 6. Parse response and check JSON output capability
    let resp_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    let model_name = resp_json
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or(&effective_model);

    // Extract the AI's message content (support thinking models that use "reasoning" field)
    let content = resp_json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|choice| {
            choice.get("message").and_then(|m| {
                m.get("content")
                    .and_then(|c| c.as_str())
                    .filter(|s| !s.is_empty())
                    .or_else(|| m.get("reasoning").and_then(|r| r.as_str()))
            })
        })
        .unwrap_or("");

    // 7. Validate JSON output capability
    let trimmed = content.trim();
    let json_ok = if trimmed.is_empty() {
        false
    } else {
        serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
            || extract_json_block(trimmed)
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .is_some()
    };

    if json_ok {
        Ok(format!("✓ {} — JSON output OK", model_name))
    } else {
        Ok(format!("⚠ {} — connected but JSON output may be unreliable (got: {})", model_name, &trimmed[..trimmed.len().min(80)]))
    }
}

/// Extract JSON content from markdown code blocks (```json ... ``` or ``` ... ```)
fn extract_json_block(text: &str) -> Option<String> {
    let start_marker = if text.contains("```json") { "```json" } else if text.contains("```") { "```" } else { return None };
    let start = text.find(start_marker)? + start_marker.len();
    let rest = &text[start..];
    let end = rest.find("```")?;
    Some(rest[..end].trim().to_string())
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
