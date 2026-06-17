//! Notification API endpoints for StockenBoard server mode.
//!
//! Provides REST endpoints for notification rules, channels, history, and cooldown management.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;

use crate::core_state::CoreState;
use crate::db::{NotificationChannelRow, NotificationHistoryRow, NotificationRuleRow};
use crate::notifications::models::{
    CreateRuleRequest, SaveChannelRequest, TelegramConfig, UpdateRuleRequest, WebhookConfig,
};

use super::{ApiError, ApiResponse};

// ─── Router ─────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<CoreState>> {
    Router::new()
        .route("/notifications/rules", get(list_rules).post(create_rule))
        .route(
            "/notifications/rules/:id",
            put(update_rule).delete(delete_rule),
        )
        .route("/notifications/rules/:id/toggle", post(toggle_rule))
        .route(
            "/notifications/channels",
            get(list_channels).post(save_channel),
        )
        .route("/notifications/channels/:id", delete(delete_channel))
        .route("/notifications/channels/:id/test", post(test_channel))
        .route("/notifications/history", get(get_history))
        .route(
            "/notifications/cooldown",
            get(get_cooldown).put(set_cooldown),
        )
}

// ─── Request/Response Types ─────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ToggleBody {
    enabled: bool,
}

#[derive(Deserialize)]
struct HistoryQuery {
    rule_id: Option<i64>,
    from: Option<i64>,
    to: Option<i64>,
    limit: Option<i64>,
}

#[derive(Deserialize)]
struct CooldownBody {
    seconds: u64,
}

// ─── Rule Endpoints ─────────────────────────────────────────────────────────────

async fn create_rule(
    State(state): State<Arc<CoreState>>,
    Json(rule): Json<CreateRuleRequest>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<serde_json::Value>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    // Validate AI config when condition_type is "ai"
    let threshold = if rule.condition_type == "ai" {
        let ai_config = rule
            .ai_config
            .as_ref()
            .ok_or_else(|| {
                ApiError::bad_request("ai_config is required when condition_type is \"ai\"")
            })?;
        ai_config.validate().map_err(ApiError::bad_request)?;
        0.0
    } else {
        rule.threshold
    };

    let channel_ids_json = serde_json::to_string(&rule.channel_ids)
        .map_err(|e| ApiError::internal(format!("Failed to serialize channel_ids: {}", e)))?;
    let cooldown = rule.cooldown_secs.unwrap_or(300) as i64;
    let ai_config_json = rule
        .ai_config
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| ApiError::internal(format!("Failed to serialize ai_config: {}", e)))?;

    // For AI rules: use subscription_ids to set both subscription_ids column and subscription_id
    // For threshold rules: ignore subscription_ids, use subscription_id directly
    let (effective_subscription_id, subscription_ids_json) = if rule.condition_type == "ai" {
        if let Some(ref ids) = rule.subscription_ids {
            if !ids.is_empty() {
                let first_id = ids[0];
                let json = serde_json::to_string(ids)
                    .map_err(|e| ApiError::internal(format!("Failed to serialize subscription_ids: {}", e)))?;
                (first_id, Some(json))
            } else {
                (rule.subscription_id, None)
            }
        } else {
            (rule.subscription_id, None)
        }
    } else {
        (rule.subscription_id, None)
    };

    let id = state
        .db
        .create_notification_rule(
            &rule.name,
            effective_subscription_id,
            &rule.condition_type,
            threshold,
            &channel_ids_json,
            cooldown,
            ai_config_json.as_deref(),
            subscription_ids_json.as_deref(),
        )
        .map_err(ApiError::internal)?;

    state.notification_engine.reload_rules().await;

    if rule.condition_type == "ai" {
        state.ai_scheduler.upsert_rule(id).await;
    }

    Ok(ApiResponse::created(serde_json::json!({ "id": id })))
}

async fn list_rules(
    State(state): State<Arc<CoreState>>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<Vec<NotificationRuleRow>>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    let rules = state.db.list_notification_rules().map_err(ApiError::internal)?;
    Ok(ApiResponse::ok(rules))
}

async fn update_rule(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<i64>,
    Json(rule): Json<UpdateRuleRequest>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<serde_json::Value>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    // Validate AI config if provided
    if let Some(Some(ref ai_config)) = rule.ai_config {
        ai_config.validate().map_err(ApiError::bad_request)?;
    }

    // If switching to AI type, ensure ai_config is provided
    if let Some(ref ct) = rule.condition_type {
        if ct == "ai" {
            match &rule.ai_config {
                Some(Some(_)) => {}
                _ => {
                    return Err(ApiError::bad_request(
                        "ai_config is required when condition_type is \"ai\"",
                    ));
                }
            }
        }
    }

    let channel_ids_json = rule
        .channel_ids
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| ApiError::internal(format!("Failed to serialize channel_ids: {}", e)))?;

    let ai_config_json: Option<Option<String>> = match &rule.ai_config {
        Some(Some(cfg)) => {
            let json = serde_json::to_string(cfg)
                .map_err(|e| ApiError::internal(format!("Failed to serialize ai_config: {}", e)))?;
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
                        .map_err(|e| ApiError::internal(format!("Failed to serialize subscription_ids: {}", e)))?;
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

    state
        .db
        .update_notification_rule(
            id,
            rule.name.as_deref(),
            rule.condition_type.as_deref(),
            threshold,
            channel_ids_json.as_deref(),
            rule.cooldown_secs.map(|s| s as i64),
            ai_config_json.as_ref().map(|opt| opt.as_deref()),
            subscription_ids_param.as_ref().map(|opt| opt.as_deref()),
            subscription_id_param,
        )
        .map_err(ApiError::internal)?;

    state.notification_engine.reload_rules().await;

    // Notify AI scheduler about the update
    match &rule.ai_config {
        Some(None) => {
            state.ai_scheduler.remove_rule(id).await;
        }
        Some(Some(_)) => {
            state.ai_scheduler.upsert_rule(id).await;
        }
        None => {
            if let Some(ref ct) = rule.condition_type {
                if ct != "ai" {
                    state.ai_scheduler.remove_rule(id).await;
                }
            }
        }
    }

    Ok(ApiResponse::ok(serde_json::json!({ "success": true })))
}

async fn delete_rule(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<i64>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<serde_json::Value>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    state
        .db
        .delete_notification_rule(id)
        .map_err(ApiError::internal)?;
    state.notification_engine.reload_rules().await;
    state.ai_scheduler.remove_rule(id).await;
    Ok(ApiResponse::ok(serde_json::json!({ "success": true })))
}

async fn toggle_rule(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<i64>,
    Json(body): Json<ToggleBody>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<serde_json::Value>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    state
        .db
        .toggle_notification_rule(id, body.enabled)
        .map_err(ApiError::internal)?;
    state.notification_engine.reload_rules().await;

    if body.enabled {
        state.ai_scheduler.upsert_rule(id).await;
    } else {
        state.ai_scheduler.remove_rule(id).await;
    }

    Ok(ApiResponse::ok(serde_json::json!({ "success": true })))
}

// ─── Channel Endpoints ──────────────────────────────────────────────────────────

async fn save_channel(
    State(state): State<Arc<CoreState>>,
    Json(channel): Json<SaveChannelRequest>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<serde_json::Value>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    let id = match channel.channel_type.as_str() {
        "telegram" => {
            let config: TelegramConfig = serde_json::from_str(&channel.config)
                .map_err(|e| ApiError::bad_request(format!("Invalid Telegram config: {}", e)))?;
            if config.bot_token.is_empty() || config.chat_id.is_empty() {
                return Err(ApiError::bad_request(
                    "Bot Token and Chat ID must not be empty",
                ));
            }
            // Encrypt bot_token before storing
            let encrypted_token = crate::notifications::crypto::encrypt_token(&config.bot_token)
                .map_err(ApiError::internal)?;
            let stored_config = serde_json::json!({
                "bot_token": encrypted_token,
                "chat_id": config.chat_id,
            });
            state
                .db
                .create_notification_channel(
                    &channel.channel_type,
                    &channel.name,
                    &stored_config.to_string(),
                )
                .map_err(ApiError::internal)?
        }
        "webhook" => {
            let config: WebhookConfig = serde_json::from_str(&channel.config)
                .map_err(|e| ApiError::bad_request(format!("Invalid Webhook config: {}", e)))?;
            if config.url.is_empty() {
                return Err(ApiError::bad_request("Webhook URL must not be empty"));
            }
            state
                .db
                .create_notification_channel(
                    &channel.channel_type,
                    &channel.name,
                    &channel.config,
                )
                .map_err(ApiError::internal)?
        }
        _ => {
            return Err(ApiError::bad_request(format!(
                "Unsupported channel type: {}",
                channel.channel_type
            )));
        }
    };

    Ok(ApiResponse::created(serde_json::json!({ "id": id })))
}

async fn list_channels(
    State(state): State<Arc<CoreState>>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<Vec<NotificationChannelRow>>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    let channels = state
        .db
        .list_notification_channels()
        .map_err(ApiError::internal)?;
    Ok(ApiResponse::ok(channels))
}

async fn delete_channel(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<i64>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<serde_json::Value>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    state
        .db
        .delete_notification_channel(id)
        .map_err(ApiError::internal)?;
    Ok(ApiResponse::ok(serde_json::json!({ "success": true })))
}

async fn test_channel(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<i64>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<serde_json::Value>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    let channels = state
        .db
        .list_notification_channels()
        .map_err(ApiError::internal)?;
    let channel = channels
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| ApiError::not_found(format!("Channel {} not found", id)))?;

    let client = reqwest::Client::new();

    match channel.channel_type.as_str() {
        "telegram" => {
            let stored_config: serde_json::Value = serde_json::from_str(&channel.config)
                .map_err(|e| ApiError::internal(format!("Failed to parse config: {}", e)))?;
            let encrypted_token = stored_config["bot_token"]
                .as_str()
                .ok_or_else(|| ApiError::internal("Missing bot_token in config"))?;
            let chat_id = stored_config["chat_id"]
                .as_str()
                .ok_or_else(|| ApiError::internal("Missing chat_id in config"))?;
            let bot_token = crate::notifications::crypto::decrypt_token(encrypted_token)
                .map_err(ApiError::internal)?;
            let config = TelegramConfig {
                bot_token,
                chat_id: chat_id.to_string(),
            };
            let test_message =
                "🔔 StockenBoard Test Notification\n\nThis is a test message to confirm the Telegram channel is configured correctly.";
            crate::notifications::telegram::send_telegram(&client, &config, test_message)
                .await
                .map_err(ApiError::internal)?;
        }
        "webhook" => {
            let config: WebhookConfig = serde_json::from_str(&channel.config)
                .map_err(|e| ApiError::internal(format!("Failed to parse config: {}", e)))?;
            let test_data = crate::notifications::models::NotificationData {
                symbol: "TEST/USD".to_string(),
                provider: "test".to_string(),
                price: 100.0,
                condition_type: crate::notifications::models::ConditionType::PriceAbove,
                threshold: 99.0,
                rule_name: "Test Rule".to_string(),
                triggered_at: chrono::Utc::now(),
            };
            crate::notifications::webhook::send_webhook(&client, &config, &test_data)
                .await
                .map_err(ApiError::internal)?;
        }
        "local" => {
            // In server mode, emit a test event via the event bus (picked up by WebSocket clients)
            let _ = state.event_bus.send(crate::events::AppEvent::NotificationTriggered(
                crate::events::NotificationTriggeredPayload {
                    rule_name: "Test Notification".to_string(),
                    symbol: "TEST/USD".to_string(),
                    provider: "test".to_string(),
                    price: 0.0,
                    condition_type: "price_above".to_string(),
                    threshold: 0.0,
                    triggered_at: chrono::Utc::now().timestamp(),
                    is_ai: false,
                    ai_reason: None,
                },
            ));
        }
        "system" => {
            // System (OS) notifications are not available in web server mode
            return Err(ApiError::bad_request(
                "System notifications are only available in desktop mode",
            ));
        }
        _ => {
            return Err(ApiError::bad_request(format!(
                "Unsupported channel type: {}",
                channel.channel_type
            )));
        }
    }

    Ok(ApiResponse::ok(
        serde_json::json!({ "success": true, "message": "Test notification sent" }),
    ))
}

// ─── History Endpoint ───────────────────────────────────────────────────────────

async fn get_history(
    State(state): State<Arc<CoreState>>,
    Query(params): Query<HistoryQuery>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<Vec<NotificationHistoryRow>>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    let history = state
        .db
        .query_notification_history(params.rule_id, params.from, params.to, params.limit)
        .map_err(ApiError::internal)?;
    Ok(ApiResponse::ok(history))
}

// ─── Cooldown Endpoints ─────────────────────────────────────────────────────────

async fn get_cooldown(
    State(state): State<Arc<CoreState>>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<u64>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    let secs = state.global_cooldown.get_cooldown();
    Ok(ApiResponse::ok(secs))
}

async fn set_cooldown(
    State(state): State<Arc<CoreState>>,
    Json(body): Json<CooldownBody>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<serde_json::Value>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    // Persist to DB
    state
        .db
        .set_setting(
            "notification_global_cooldown",
            &body.seconds.to_string(),
        )
        .map_err(ApiError::internal)?;
    // Update in-memory value
    state.global_cooldown.set_cooldown(body.seconds);
    Ok(ApiResponse::ok(
        serde_json::json!({ "seconds": body.seconds }),
    ))
}
