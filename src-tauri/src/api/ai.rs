//! AI configuration API endpoints.
//!
//! Provides:
//! - `POST /ai/config` — save AI provider configuration
//! - `GET /ai/config` — get AI provider configuration
//! - `POST /ai/test` — test AI connection
//! - `GET /ai/models` — list available AI models

use std::sync::Arc;

use axum::{
    extract::{Json, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::Deserialize;

use crate::api::{ApiError, ApiResponse};
use crate::core_state::CoreState;

// ─── Request / Query Types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SaveConfigRequest {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TestConnectionRequest {
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListModelsQuery {
    pub base_url: String,
    pub api_key: Option<String>,
}

// ─── Router ─────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<CoreState>> {
    Router::new()
        .route("/ai/config", get(get_config).post(save_config))
        .route("/ai/test", post(test_connection))
        .route("/ai/models", get(list_models))
}

// ─── Handlers ───────────────────────────────────────────────────────────────────

/// POST /ai/config
/// Save AI provider configuration (base_url, model, optional api_key).
async fn save_config(
    State(state): State<Arc<CoreState>>,
    Json(body): Json<SaveConfigRequest>,
) -> axum::response::Response {
    match state
        .db
        .save_ai_provider_config(&body.base_url, &body.model, body.api_key.as_deref())
    {
        Ok(()) => ApiResponse::ok(serde_json::json!({ "success": true })).into_response(),
        Err(e) => ApiError::bad_request(e).into_response(),
    }
}

/// GET /ai/config
/// Get the current AI provider configuration (api_key is masked — only `has_api_key` is returned).
async fn get_config(State(state): State<Arc<CoreState>>) -> axum::response::Response {
    match state.db.load_ai_provider_config() {
        Ok(Some(config)) => {
            let response = serde_json::json!({
                "base_url": config.base_url,
                "model": config.model,
                "has_api_key": config.api_key.is_some(),
            });
            ApiResponse::ok(response).into_response()
        }
        Ok(None) => ApiResponse::ok(serde_json::Value::Null).into_response(),
        Err(e) => ApiError::internal(e).into_response(),
    }
}

/// POST /ai/test
/// Test AI connection using the saved config (or override with request body values).
async fn test_connection(
    State(state): State<Arc<CoreState>>,
    Json(body): Json<TestConnectionRequest>,
) -> axum::response::Response {
    // Load saved config as a base, then allow overrides from request body
    let config = match state.db.load_ai_provider_config() {
        Ok(Some(c)) => c,
        Ok(None) => {
            // If no saved config, the request body must provide base_url and model
            let base_url = match &body.base_url {
                Some(u) if !u.is_empty() => u.clone(),
                _ => {
                    return ApiError::bad_request(
                        "AI provider not configured and no base_url provided",
                    )
                    .into_response()
                }
            };
            let model = match &body.model {
                Some(m) if !m.is_empty() => m.clone(),
                _ => {
                    return ApiError::bad_request(
                        "AI provider not configured and no model provided",
                    )
                    .into_response()
                }
            };
            crate::notifications::models::AiProviderConfig {
                base_url,
                model,
                api_key: body.api_key.clone(),
            }
        }
        Err(e) => return ApiError::internal(e).into_response(),
    };

    // Apply overrides from request body
    let base_url = body
        .base_url
        .as_ref()
        .filter(|u| !u.is_empty())
        .unwrap_or(&config.base_url)
        .clone();
    let model = body
        .model
        .as_ref()
        .filter(|m| !m.is_empty())
        .unwrap_or(&config.model)
        .clone();
    let api_key = body.api_key.clone().or(config.api_key);

    // Build the test request
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let request_body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "Hello"}],
        "max_tokens": 5
    });

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => return ApiError::internal(format!("Failed to create HTTP client: {}", e)).into_response(),
    };

    let mut request = client.post(&url).json(&request_body);
    if let Some(ref key) = api_key {
        if !key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", key));
        }
    }

    let response = match request.send().await {
        Ok(r) => r,
        Err(e) => {
            return ApiError::bad_request(format!("Connection failed: {}", e)).into_response()
        }
    };

    let status = response.status();
    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return ApiError::bad_request(format!(
            "AI API returned error (HTTP {}): {}",
            status.as_u16(),
            error_body
        ))
        .into_response();
    }

    // Parse response to extract model name
    let resp_json: serde_json::Value = match response.json().await {
        Ok(j) => j,
        Err(e) => {
            return ApiError::internal(format!("Failed to parse response: {}", e)).into_response()
        }
    };

    let model_name = resp_json
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or(&model);

    ApiResponse::ok(serde_json::json!({
        "message": format!("Connection successful! Model: {}", model_name),
        "model": model_name,
    }))
    .into_response()
}

/// GET /ai/models?base_url=&api_key=
/// List available AI models from the configured provider.
async fn list_models(
    Query(query): Query<ListModelsQuery>,
) -> axum::response::Response {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ApiError::internal(format!("Failed to create HTTP client: {}", e))
                .into_response()
        }
    };

    let trimmed_url = query.base_url.trim_end_matches('/');

    // Try Ollama native API: {base_url without /v1}/api/tags
    let ollama_base = trimmed_url.trim_end_matches("/v1");
    let ollama_url = format!("{}/api/tags", ollama_base);

    if let Ok(resp) = client.get(&ollama_url).send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(models) = json.get("models").and_then(|m| m.as_array()) {
                    let names: Vec<String> = models
                        .iter()
                        .filter_map(|m| {
                            m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string())
                        })
                        .collect();
                    if !names.is_empty() {
                        return ApiResponse::ok(names).into_response();
                    }
                }
            }
        }
    }

    // Try OpenAI-compatible /models endpoint
    let openai_url = format!("{}/models", trimmed_url);
    let mut req = client.get(&openai_url);
    if let Some(ref key) = query.api_key {
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
                        .filter_map(|m| {
                            m.get("id").and_then(|n| n.as_str()).map(|s| s.to_string())
                        })
                        .collect();
                    if !names.is_empty() {
                        return ApiResponse::ok(names).into_response();
                    }
                }
            }
        }
    }

    ApiError::bad_request("Unable to retrieve model list. Please verify the URL is correct.")
        .into_response()
}
