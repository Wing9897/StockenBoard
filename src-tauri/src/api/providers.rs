//! Provider and provider-settings HTTP API endpoints.
//!
//! Routes:
//! - `GET  /providers`                — list all available providers
//! - `POST /providers/:id/enable`     — enable a provider (register with registry)
//! - `GET  /provider-settings`        — list all provider settings from DB
//! - `PUT  /provider-settings/:id`    — upsert provider settings
//! - `GET  /provider-settings/:id/has-key` — check if provider has an API key configured

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;

use crate::core_state::CoreState;
use crate::providers::get_all_provider_info;

use super::{ApiError, ApiResponse};

// ─── Request Bodies ─────────────────────────────────────────────────────────────

/// Body for `POST /providers/:id/enable`
#[derive(Debug, Deserialize)]
pub struct EnableProviderBody {
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
}

/// Body for `PUT /provider-settings/:id`
#[derive(Debug, Deserialize)]
pub struct UpsertProviderSettingsBody {
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub api_url: Option<String>,
    pub refresh_interval: Option<i64>,
    pub connection_type: Option<String>,
    pub record_from_hour: Option<i64>,
    pub record_to_hour: Option<i64>,
}

// ─── Router ─────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<CoreState>> {
    Router::new()
        .route("/providers", get(list_providers))
        .route("/providers/{id}/enable", post(enable_provider))
        .route("/provider-settings", get(list_settings))
        .route("/provider-settings/{id}", put(upsert_settings))
        .route("/provider-settings/{id}/has-key", get(has_key))
}

// ─── Handlers ───────────────────────────────────────────────────────────────────

/// `GET /providers` — return the static list of all available providers.
async fn list_providers() -> impl axum::response::IntoResponse {
    let providers = get_all_provider_info();
    ApiResponse::ok(providers)
}

/// `POST /providers/:id/enable` — enable a provider in the registry.
///
/// Reads stored api_url from DB settings, then calls `registry.update_provider(...)`.
/// Finally triggers a polling reload.
async fn enable_provider(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<String>,
    Json(body): Json<EnableProviderBody>,
) -> impl axum::response::IntoResponse {
    // Look up existing api_url from DB settings
    let api_url = state
        .db
        .get_provider_settings(&id)
        .ok()
        .flatten()
        .and_then(|s| s.api_url.filter(|u| !u.is_empty()));

    state
        .registry
        .update_provider(&id, body.api_key, body.api_secret, api_url)
        .await;

    state.polling.reload();

    ApiResponse::ok(serde_json::json!({ "enabled": true }))
}

/// `GET /provider-settings` — list all provider settings rows from the database.
async fn list_settings(
    State(state): State<Arc<CoreState>>,
) -> impl axum::response::IntoResponse {
    match state.db.list_provider_settings() {
        Ok(settings) => Ok(ApiResponse::ok(settings)),
        Err(e) => Err(ApiError::internal(e)),
    }
}

/// `PUT /provider-settings/:id` — upsert provider settings, update registry, and reload polling.
async fn upsert_settings(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<String>,
    Json(body): Json<UpsertProviderSettingsBody>,
) -> impl axum::response::IntoResponse {
    let connection_type = body.connection_type.as_deref().unwrap_or("rest");

    let result = state.db.upsert_provider_settings(
        &id,
        body.api_key.as_deref(),
        body.api_secret.as_deref(),
        body.api_url.as_deref(),
        body.refresh_interval,
        connection_type,
        body.record_from_hour,
        body.record_to_hour,
    );

    match result {
        Ok(()) => {
            // Also update the registry so the provider is available for fetching
            state
                .registry
                .update_provider(
                    &id,
                    body.api_key,
                    body.api_secret,
                    body.api_url,
                )
                .await;

            state.polling.reload();

            Ok(ApiResponse::ok(serde_json::json!({ "updated": true })))
        }
        Err(e) => Err(ApiError::internal(e)),
    }
}

/// `GET /provider-settings/:id/has-key` — check if the provider has an API key configured.
async fn has_key(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<String>,
) -> impl axum::response::IntoResponse {
    let exists = state.db.has_api_key(&id);
    ApiResponse::ok(serde_json::json!({ "has_key": exists }))
}
