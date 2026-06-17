//! Subscription management API endpoints.
//!
//! Provides CRUD operations for subscriptions:
//! - `GET /subscriptions` — list all (with optional `?type=` filter)
//! - `POST /subscriptions` — add a single subscription
//! - `POST /subscriptions/batch` — add multiple subscriptions
//! - `PUT /subscriptions/:id` — update a subscription
//! - `DELETE /subscriptions/:id` — remove a subscription
//! - `DELETE /subscriptions/batch` — remove multiple subscriptions

use std::sync::Arc;

use axum::{
    extract::{Json, Path, Query, State},
    routing::{get, post, put},
    Router,
};
use serde::Deserialize;

use crate::api::{ApiError, ApiResponse};
use crate::core_state::CoreState;
use crate::db::BatchAddResult;
use crate::providers::normalize_symbol;

// ─── Query / Request Types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(rename = "type")]
    pub sub_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddSubscriptionRequest {
    pub sub_type: String,
    pub symbol: String,
    pub display_name: Option<String>,
    pub provider_id: String,
    pub asset_type: String,
    pub pool_address: Option<String>,
    pub token_from: Option<String>,
    pub token_to: Option<String>,
}

use crate::db::BatchAddItem;

#[derive(Debug, Deserialize)]
pub struct UpdateSubscriptionRequest {
    pub symbol: String,
    pub display_name: Option<String>,
    pub provider_id: String,
    pub asset_type: String,
}

#[derive(Debug, Deserialize)]
pub struct BatchRemoveRequest {
    pub ids: Vec<i64>,
}

// ─── Router ─────────────────────────────────────────────────────────────────────

// ─── Toggle Record / Record Hours Types ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ToggleRecordRequest {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct SetRecordHoursRequest {
    pub from_hour: Option<i64>,
    pub to_hour: Option<i64>,
}

// ─── Router ─────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<CoreState>> {
    Router::new()
        .route("/subscriptions", get(list_subscriptions).post(add_subscription))
        .route("/subscriptions/batch", post(add_batch).delete(remove_batch))
        .route("/subscriptions/:id", put(update_subscription).delete(remove_subscription))
        .route("/subscriptions/:id/toggle-record", post(toggle_record))
        .route("/subscriptions/:id/record-hours", axum::routing::put(set_record_hours))
}

// ─── Handlers ───────────────────────────────────────────────────────────────────

/// GET /subscriptions?type=<sub_type>
/// Lists all subscriptions, optionally filtered by type.
async fn list_subscriptions(
    State(state): State<Arc<CoreState>>,
    Query(query): Query<ListQuery>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    let result = match &query.sub_type {
        Some(t) => state.db.list_subscriptions(t),
        None => state.db.list_all_subscriptions(),
    };

    match result {
        Ok(subs) => Ok(ApiResponse::ok(subs).into_response()),
        Err(e) => Err(ApiError::internal(e).into_response()),
    }
}

/// POST /subscriptions
/// Add a single subscription.
async fn add_subscription(
    State(state): State<Arc<CoreState>>,
    Json(body): Json<AddSubscriptionRequest>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    let normalized = if body.sub_type == "dex" {
        body.symbol.clone()
    } else {
        normalize_symbol(&body.symbol, &body.asset_type)
    };

    match state.db.add_subscription(
        &body.sub_type,
        &normalized,
        body.display_name.as_deref(),
        &body.provider_id,
        &body.asset_type,
        body.pool_address.as_deref(),
        body.token_from.as_deref(),
        body.token_to.as_deref(),
    ) {
        Ok(id) => {
            state.polling.reload();
            Ok(ApiResponse::created(serde_json::json!({ "id": id })).into_response())
        }
        Err(e) => Err(ApiError::bad_request(e).into_response()),
    }
}

/// POST /subscriptions/batch
/// Add multiple subscriptions at once (asset type only).
async fn add_batch(
    State(state): State<Arc<CoreState>>,
    Json(items): Json<Vec<BatchAddItem>>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    let mut duplicates = Vec::new();

    for item in &items {
        let normalized = normalize_symbol(&item.symbol, &item.asset_type);
        match state.db.add_subscription(
            "asset",
            &normalized,
            item.display_name.as_deref(),
            &item.provider_id,
            &item.asset_type,
            None,
            None,
            None,
        ) {
            Ok(_) => succeeded.push(normalized),
            Err(e) if e.contains("already exists") => duplicates.push(normalized),
            Err(_) => failed.push(normalized),
        }
    }

    if !succeeded.is_empty() {
        state.polling.reload();
    }

    Ok(ApiResponse::ok(BatchAddResult {
        succeeded,
        failed,
        duplicates,
    })
    .into_response())
}

/// PUT /subscriptions/:id
/// Update an existing subscription.
async fn update_subscription(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateSubscriptionRequest>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    let normalized = normalize_symbol(&body.symbol, &body.asset_type);

    match state.db.update_subscription(
        id,
        &normalized,
        body.display_name.as_deref(),
        &body.provider_id,
        &body.asset_type,
    ) {
        Ok(()) => {
            state.polling.reload();
            Ok(ApiResponse::ok(serde_json::json!({ "success": true })).into_response())
        }
        Err(e) => Err(ApiError::internal(e).into_response()),
    }
}

/// DELETE /subscriptions/:id
/// Remove a single subscription.
async fn remove_subscription(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<i64>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    match state.db.remove_subscription(id) {
        Ok(()) => {
            state.polling.reload();
            Ok(ApiResponse::ok(serde_json::json!({ "success": true })).into_response())
        }
        Err(e) => Err(ApiError::internal(e).into_response()),
    }
}

/// DELETE /subscriptions/batch
/// Remove multiple subscriptions at once.
async fn remove_batch(
    State(state): State<Arc<CoreState>>,
    Json(body): Json<BatchRemoveRequest>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    match state.db.remove_subscriptions(&body.ids) {
        Ok(()) => {
            state.polling.reload();
            Ok(ApiResponse::ok(serde_json::json!({ "success": true })).into_response())
        }
        Err(e) => Err(ApiError::internal(e).into_response()),
    }
}

/// POST /subscriptions/:id/toggle-record
/// Enable or disable price recording for a subscription.
async fn toggle_record(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<i64>,
    Json(body): Json<ToggleRecordRequest>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    match state.db.toggle_record(id, body.enabled) {
        Ok(()) => {
            state.polling.reload();
            Ok(ApiResponse::ok(serde_json::json!({ "success": true })).into_response())
        }
        Err(e) => Err(ApiError::internal(e).into_response()),
    }
}

/// PUT /subscriptions/:id/record-hours
/// Set recording hours for a subscription.
async fn set_record_hours(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<i64>,
    Json(body): Json<SetRecordHoursRequest>,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;

    match state.db.set_record_hours(id, body.from_hour, body.to_hour) {
        Ok(()) => Ok(ApiResponse::ok(serde_json::json!({ "success": true })).into_response()),
        Err(e) => Err(ApiError::internal(e).into_response()),
    }
}
