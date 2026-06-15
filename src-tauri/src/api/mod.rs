//! HTTP API router and shared response types for StockenBoard server mode.
//!
//! Provides:
//! - `build_router(state)` — constructs the full Axum router with CORS and 404 fallback
//! - `ApiResponse<T>` — success envelope `{ "data": T }`
//! - `ApiError` / `ApiErrorBody` — error envelope `{ "error": { "code", "message" } }`

use std::path::Path;
use std::sync::Arc;

use axum::{http::StatusCode, response::IntoResponse, Json, Router};
use serde::Serialize;
use tower_http::cors::CorsLayer;

use crate::core_state::CoreState;

// Submodules for each resource group (will be created in tasks 4.2–4.8)
pub mod subscriptions;
pub mod views;
pub mod providers;
pub mod notifications;
pub mod ai;
pub mod prices;
pub mod system;
pub mod ws;
pub mod static_files;

// ─── Response Envelope Types ────────────────────────────────────────────────────

/// Success response envelope: `{ "data": T }`
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }

    /// Return a `200 OK` JSON response with the data envelope.
    pub fn ok(data: T) -> (StatusCode, Json<Self>) {
        (StatusCode::OK, Json(Self::new(data)))
    }

    /// Return a `201 Created` JSON response with the data envelope.
    pub fn created(data: T) -> (StatusCode, Json<Self>) {
        (StatusCode::CREATED, Json(Self::new(data)))
    }
}

/// Error response envelope: `{ "error": { "code": string, "message": string } }`
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: ApiErrorBody,
}

/// Inner body of an API error response.
#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ApiErrorBody {
                code: code.into(),
                message: message.into(),
            },
        }
    }

    pub fn not_found(message: impl Into<String>) -> (StatusCode, Json<Self>) {
        (
            StatusCode::NOT_FOUND,
            Json(Self::new("not_found", message)),
        )
    }

    pub fn bad_request(message: impl Into<String>) -> (StatusCode, Json<Self>) {
        (
            StatusCode::BAD_REQUEST,
            Json(Self::new("bad_request", message)),
        )
    }

    pub fn internal(message: impl Into<String>) -> (StatusCode, Json<Self>) {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Self::new("internal_error", message)),
        )
    }
}

// ─── Router Builder ─────────────────────────────────────────────────────────────

/// Build the full API router with all routes, CORS permissive layer, and 404 fallback.
///
/// The router is nested under `/api` so that static file serving can occupy `/` later.
/// Use [`build_router_with_static`] to include SPA static file serving.
pub fn build_router(state: Arc<CoreState>) -> Router {
    let api_routes = Router::new()
        // Sub-routers will be merged here in tasks 4.2–4.8
        .merge(subscriptions::router())
        .merge(views::router())
        .merge(providers::router())
        .merge(notifications::router())
        .merge(ai::router())
        .merge(prices::router())
        .merge(system::router())
        .merge(ws::router())
        .fallback(api_fallback)
        .with_state(state);

    Router::new()
        .nest("/api", api_routes)
        .layer(CorsLayer::permissive())
}

/// Build the full application router with API routes AND static file serving.
///
/// This wires up:
/// 1. `/api/*` routes (take precedence)
/// 2. Static file serving with SPA fallback (serves built SPA from `static_dir`)
///
/// Static file serving includes:
/// - Correct MIME types based on file extension
/// - `Cache-Control: public, max-age=31536000, immutable` for hashed assets (e.g., `assets/index-a1b2c3.js`)
/// - `Cache-Control: no-cache, no-store, must-revalidate` for `index.html`
/// - SPA fallback: paths not matching API routes or static files serve `index.html`
pub fn build_router_with_static(state: Arc<CoreState>, static_dir: &Path) -> Router {
    let api_routes = Router::new()
        .merge(subscriptions::router())
        .merge(views::router())
        .merge(providers::router())
        .merge(notifications::router())
        .merge(ai::router())
        .merge(prices::router())
        .merge(system::router())
        .merge(ws::router())
        .fallback(api_fallback)
        .with_state(state);

    // Static file layer serves files from static_dir and falls back to index.html.
    // Uses static_file_service which applies cache header middleware:
    // - Cache-Control: public, max-age=31536000, immutable for hashed assets
    // - Cache-Control: no-cache, no-store, must-revalidate for index.html
    let static_service = static_files::static_file_service(static_dir);

    Router::new()
        .nest("/api", api_routes)
        .fallback_service(static_service)
        .layer(CorsLayer::permissive())
}

// ─── Fallback Handler ───────────────────────────────────────────────────────────

/// Fallback handler for unknown API routes — returns 404 JSON error body.
async fn api_fallback() -> impl IntoResponse {
    ApiError::not_found("The requested endpoint does not exist")
}
