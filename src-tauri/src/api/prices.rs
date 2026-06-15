//! Price fetching and history API endpoints.
//!
//! Provides:
//! - `GET /prices/fetch/:provider/:symbol` — fetch a single price from a provider
//! - `POST /prices/fetch-multiple` — fetch multiple prices from a provider
//! - `GET /prices/cached` — get all cached prices from polling
//! - `GET /prices/poll-ticks` — get current poll ticks per provider
//! - `GET /history/stats` — get history stats for subscription IDs
//! - `GET /history/:sub_id` — get price history for a subscription
//! - `POST /history/cleanup` — cleanup old history records
//! - `DELETE /history` — purge all history
//! - `DELETE /history/:sub_id` — delete history for a subscription

use std::sync::Arc;

use axum::{
    extract::{Json, Path, Query, State},
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use serde::Deserialize;

use crate::api::{ApiError, ApiResponse};
use crate::core_state::CoreState;

// ─── Query / Request Types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FetchMultipleRequest {
    pub provider_id: String,
    pub symbols: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    /// Comma-separated subscription IDs
    pub subscription_ids: String,
}

#[derive(Debug, Deserialize)]
pub struct CleanupRequest {
    pub retention_days: Option<i64>,
}

// ─── Response Types ─────────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct HistoryStatsResult {
    pub subscription_id: i64,
    pub total_records: i64,
    pub earliest: Option<i64>,
    pub latest: Option<i64>,
}

// ─── Router ─────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<CoreState>> {
    Router::new()
        .route("/prices/fetch/{provider}/{symbol}", get(fetch_single))
        .route("/prices/fetch-multiple", post(fetch_multiple))
        .route("/prices/cached", get(get_cached))
        .route("/prices/poll-ticks", get(get_poll_ticks))
        .route("/history/stats", get(get_stats))
        .route("/history/cleanup", post(cleanup))
        .route("/history", delete(purge_all))
        .route("/history/{sub_id}", get(get_history).delete(delete_history))
}

// ─── Handlers ───────────────────────────────────────────────────────────────────

/// GET /prices/fetch/:provider/:symbol
/// Fetch a single price from the specified provider.
async fn fetch_single(
    State(state): State<Arc<CoreState>>,
    Path((provider, symbol)): Path<(String, String)>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let provider_instance = state
        .registry
        .get_or_create(&provider, &state.db)
        .await
        .ok_or_else(|| ApiError::not_found(format!("Provider not found: {}", provider)))?;

    match provider_instance.fetch_price(&symbol).await {
        Ok(data) => Ok(ApiResponse::ok(data)),
        Err(e) => Err(ApiError::internal(e)),
    }
}

/// POST /prices/fetch-multiple
/// Fetch prices for multiple symbols from a provider (with rate limiting).
async fn fetch_multiple(
    State(state): State<Arc<CoreState>>,
    Json(body): Json<FetchMultipleRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    if body.symbols.is_empty() {
        return Err(ApiError::bad_request("symbols array must not be empty"));
    }

    match state
        .registry
        .fetch_with_limit(&body.provider_id, &body.symbols, &state.db)
        .await
    {
        Ok(data) => Ok(ApiResponse::ok(data)),
        Err(e) => Err(ApiError::internal(e)),
    }
}

/// GET /prices/cached
/// Return all currently cached prices from polling.
async fn get_cached(
    State(state): State<Arc<CoreState>>,
) -> impl IntoResponse {
    let cache = state.polling.cache.read().await;
    ApiResponse::ok(cache.clone())
}

/// GET /prices/poll-ticks
/// Return current poll tick info per provider.
async fn get_poll_ticks(
    State(state): State<Arc<CoreState>>,
) -> impl IntoResponse {
    let ticks = state.polling.ticks.read().await;
    ApiResponse::ok(ticks.clone())
}

/// GET /history/stats?subscription_ids=1,2,3
/// Get history statistics for specified subscription IDs.
async fn get_stats(
    State(state): State<Arc<CoreState>>,
    Query(query): Query<StatsQuery>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let ids: Vec<i64> = query
        .subscription_ids
        .split(',')
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .collect();

    if ids.is_empty() {
        return Err(ApiError::bad_request(
            "subscription_ids must contain at least one valid ID",
        ));
    }

    let mut results = Vec::new();
    for sid in ids {
        match state.db.get_history_stats(sid) {
            Ok(stats) => results.push(HistoryStatsResult {
                subscription_id: sid,
                total_records: stats.total,
                earliest: stats.oldest,
                latest: stats.newest,
            }),
            Err(e) => return Err(ApiError::internal(e)),
        }
    }

    Ok(ApiResponse::ok(results))
}

/// GET /history/:sub_id?from=&to=&limit=
/// Get price history records for a subscription.
async fn get_history(
    State(state): State<Arc<CoreState>>,
    Path(sub_id): Path<i64>,
    Query(query): Query<HistoryQuery>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let limit = query.limit.unwrap_or(500);

    match state.db.get_price_history(sub_id, query.from, query.to, limit) {
        Ok(rows) => Ok(ApiResponse::ok(rows)),
        Err(e) => Err(ApiError::internal(e)),
    }
}

/// POST /history/cleanup
/// Delete history records older than retention_days (default 90).
async fn cleanup(
    State(state): State<Arc<CoreState>>,
    Json(body): Json<CleanupRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let days = body.retention_days.unwrap_or(90);
    let cutoff = chrono::Utc::now().timestamp() - (days * 86400);

    match state.db.cleanup_history(cutoff) {
        Ok(deleted) => Ok(ApiResponse::ok(serde_json::json!({ "deleted": deleted }))),
        Err(e) => Err(ApiError::internal(e)),
    }
}

/// DELETE /history
/// Purge all price history records.
async fn purge_all(
    State(state): State<Arc<CoreState>>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    match state.db.purge_all_history() {
        Ok(()) => Ok(ApiResponse::ok(serde_json::json!({ "success": true }))),
        Err(e) => Err(ApiError::internal(e)),
    }
}

/// DELETE /history/:sub_id
/// Delete all history for a specific subscription.
async fn delete_history(
    State(state): State<Arc<CoreState>>,
    Path(sub_id): Path<i64>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    match state.db.delete_history_for_subscription(sub_id) {
        Ok(deleted) => Ok(ApiResponse::ok(serde_json::json!({ "deleted": deleted }))),
        Err(e) => Err(ApiError::internal(e)),
    }
}
