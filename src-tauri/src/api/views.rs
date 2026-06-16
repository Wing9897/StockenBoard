//! View management endpoints.
//!
//! - `GET  /views?type=`             — list views by type
//! - `POST /views`                   — create a new view
//! - `GET  /views/sub-counts`        — subscription count per view
//! - `PUT  /views/:id`               — rename a view
//! - `DELETE /views/:id`             — delete a view
//! - `GET  /views/:id/subscription-ids` — subscription IDs for a view
//! - `POST /views/:id/subscriptions` — add subscription to view
//! - `DELETE /views/:id/subscriptions/:sub_id` — remove subscription from view

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get},
    Json, Router,
};
use serde::Deserialize;

use crate::core_state::CoreState;
use crate::db::ViewSubCount;

use super::{ApiError, ApiResponse};

// ─── Query / Body types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListViewsQuery {
    #[serde(rename = "type", default)]
    pub view_type: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateViewBody {
    pub name: String,
    #[serde(rename = "type", default)]
    pub view_type: String,
}

#[derive(Debug, Deserialize)]
pub struct RenameViewBody {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AddSubBody {
    pub subscription_id: i64,
}

// ─── Router ─────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<CoreState>> {
    Router::new()
        .route("/views", get(list_views).post(create_view))
        .route("/views/sub-counts", get(get_view_sub_counts))
        .route("/views/:id", axum::routing::put(rename_view).delete(delete_view))
        .route("/views/:id/subscriptions", axum::routing::post(add_sub_to_view))
        .route("/views/:id/subscription-ids", get(get_view_subscription_ids))
        .route(
            "/views/:id/subscriptions/:sub_id",
            delete(remove_sub_from_view),
        )
}

// ─── Handlers ───────────────────────────────────────────────────────────────────

async fn list_views(
    State(state): State<Arc<CoreState>>,
    Query(params): Query<ListViewsQuery>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<Vec<crate::db::ViewRow>>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    state
        .db
        .list_views(&params.view_type)
        .map(ApiResponse::ok)
        .map_err(ApiError::internal)
}

async fn create_view(
    State(state): State<Arc<CoreState>>,
    Json(body): Json<CreateViewBody>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<serde_json::Value>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    if body.name.trim().is_empty() {
        return Err(ApiError::bad_request("name is required"));
    }
    state
        .db
        .create_view(&body.name, &body.view_type)
        .map(|id| ApiResponse::created(serde_json::json!({ "id": id })))
        .map_err(ApiError::internal)
}

async fn rename_view(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<i64>,
    Json(body): Json<RenameViewBody>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<serde_json::Value>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    if body.name.trim().is_empty() {
        return Err(ApiError::bad_request("name is required"));
    }
    state
        .db
        .rename_view(id, &body.name)
        .map(|_| ApiResponse::ok(serde_json::json!({ "success": true })))
        .map_err(ApiError::internal)
}

async fn delete_view(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<i64>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<serde_json::Value>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    state
        .db
        .delete_view(id)
        .map(|_| ApiResponse::ok(serde_json::json!({ "success": true })))
        .map_err(ApiError::internal)
}

async fn add_sub_to_view(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<i64>,
    Json(body): Json<AddSubBody>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<serde_json::Value>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    state
        .db
        .add_sub_to_view(id, body.subscription_id)
        .map(|_| ApiResponse::created(serde_json::json!({ "success": true })))
        .map_err(ApiError::internal)
}

async fn remove_sub_from_view(
    State(state): State<Arc<CoreState>>,
    Path((id, sub_id)): Path<(i64, i64)>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<serde_json::Value>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    state
        .db
        .remove_sub_from_view(id, sub_id)
        .map(|_| ApiResponse::ok(serde_json::json!({ "success": true })))
        .map_err(ApiError::internal)
}

async fn get_view_sub_counts(
    State(state): State<Arc<CoreState>>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<Vec<ViewSubCount>>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    state
        .db
        .get_view_sub_counts()
        .map(ApiResponse::ok)
        .map_err(ApiError::internal)
}

async fn get_view_subscription_ids(
    State(state): State<Arc<CoreState>>,
    Path(id): Path<i64>,
) -> Result<
    (axum::http::StatusCode, Json<ApiResponse<Vec<i64>>>),
    (axum::http::StatusCode, Json<ApiError>),
> {
    state
        .db
        .get_view_subscription_ids(id)
        .map(ApiResponse::ok)
        .map_err(ApiError::internal)
}
