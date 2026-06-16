//! WebSocket handler for real-time event streaming at `/api/ws`.
//!
//! Upgrades HTTP connections to WebSocket and:
//! - Subscribes to `CoreState.event_bus` and forwards all `AppEvent` variants as JSON
//! - Handles incoming `start_ws_stream` / `stop_ws_stream` commands for provider WS streams
//! - Cleans up resources (subscriptions, WS tasks) on client disconnect

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::core_state::CoreState;
use crate::events::AppEvent;
use crate::providers::{create_ws_provider, WsTickerUpdate};

// ─── WsMessage Envelope ─────────────────────────────────────────────────────────

/// Outgoing WebSocket message envelope sent to clients.
#[derive(Debug, Serialize)]
struct WsMessage {
    #[serde(rename = "type")]
    msg_type: String,
    data: serde_json::Value,
    timestamp: i64,
}

impl WsMessage {
    fn new(msg_type: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            msg_type: msg_type.into(),
            data,
            timestamp: Utc::now().timestamp_millis(),
        }
    }

    fn from_app_event(event: &AppEvent) -> Self {
        match event {
            AppEvent::PriceUpdate {
                provider_id: _,
                data,
                record_symbols: _,
            } => WsMessage::new(
                "price-update",
                serde_json::to_value(data).unwrap_or_default(),
            ),
            AppEvent::PriceError {
                provider_id,
                symbols,
                error,
            } => {
                // Match Tauri emit format: { "provider:symbol": "error msg", ... }
                let map: std::collections::HashMap<String, String> = symbols
                    .iter()
                    .map(|s| (format!("{}:{}", provider_id, s), error.clone()))
                    .collect();
                WsMessage::new(
                    "price-error",
                    serde_json::to_value(&map).unwrap_or_default(),
                )
            }
            AppEvent::PollTick {
                provider_id,
                fetched_at,
                interval_ms,
            } => WsMessage::new(
                "poll-tick",
                serde_json::json!({
                    "provider_id": provider_id,
                    "fetched_at": fetched_at,
                    "interval_ms": interval_ms,
                }),
            ),
            AppEvent::NotificationTriggered(payload) => WsMessage::new(
                "notification-triggered",
                serde_json::to_value(payload).unwrap_or_default(),
            ),
            AppEvent::SystemNotification { title, body } => WsMessage::new(
                "system-notification",
                serde_json::json!({ "title": title, "body": body }),
            ),
            AppEvent::LogoDownloadProgress(progress) => WsMessage::new(
                "logo-download-progress",
                serde_json::to_value(progress).unwrap_or_default(),
            ),
        }
    }

    fn from_ws_ticker(update: &WsTickerUpdate) -> Self {
        WsMessage::new(
            "ws-ticker-update",
            serde_json::to_value(update).unwrap_or_default(),
        )
    }
}

// ─── Incoming Command Types ─────────────────────────────────────────────────────

/// Incoming WebSocket command from client.
#[derive(Debug, Deserialize)]
struct WsCommand {
    command: String,
    #[serde(default)]
    provider_id: Option<String>,
    #[serde(default)]
    symbols: Option<Vec<String>>,
}

// ─── Router ─────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<CoreState>> {
    Router::new()
        .route("/ws", get(ws_handler))
        .route("/ws/stream/start", axum::routing::post(start_ws_stream))
        .route("/ws/stream/stop", axum::routing::post(stop_ws_stream))
}

// ─── Handler ────────────────────────────────────────────────────────────────────

/// Upgrade HTTP connection to WebSocket.
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<CoreState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state))
}

/// Main WebSocket connection loop.
///
/// Spawns two tasks:
/// 1. **send_task** — forwards `AppEvent` and `WsTickerUpdate` messages to the client
/// 2. **recv_task** — processes incoming commands (`start_ws_stream`, `stop_ws_stream`)
///
/// On disconnect, both tasks are aborted and WS provider streams are cleaned up.
async fn handle_ws_connection(socket: WebSocket, state: Arc<CoreState>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to the shared event bus
    let mut event_rx = state.event_bus.subscribe();

    // Channel for WS ticker updates from provider streams
    let (ws_ticker_tx, ws_ticker_rx) =
        broadcast::channel::<WsTickerUpdate>(256);

    // Track active WS stream tasks for cleanup
    let ws_tasks: Arc<tokio::sync::Mutex<HashMap<String, tokio::task::JoinHandle<()>>>> =
        Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    // ─── Send task: forward event bus + WS ticker events to client ───────────────
    let send_task = tokio::spawn(async move {
        let mut ws_ticker_sub = ws_ticker_rx.resubscribe();
        loop {
            tokio::select! {
                result = event_rx.recv() => {
                    match result {
                        Ok(event) => {
                            let msg = WsMessage::from_app_event(&event);
                            let text = match serde_json::to_string(&msg) {
                                Ok(t) => t,
                                Err(_) => continue,
                            };
                            if sender.send(Message::Text(text)).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                result = ws_ticker_sub.recv() => {
                    match result {
                        Ok(update) => {
                            let msg = WsMessage::from_ws_ticker(&update);
                            let text = match serde_json::to_string(&msg) {
                                Ok(t) => t,
                                Err(_) => continue,
                            };
                            if sender.send(Message::Text(text)).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });

    // ─── Receive task: handle incoming commands ─────────────────────────────────
    let ws_tasks_clone = ws_tasks.clone();
    let ws_ticker_tx_clone = ws_ticker_tx.clone();

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    let cmd: WsCommand = match serde_json::from_str(&text) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    handle_command(cmd, &ws_tasks_clone, &ws_ticker_tx_clone).await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to finish (client disconnect or error)
    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }

    // ─── Cleanup: abort all active WS provider streams ──────────────────────────
    let mut tasks = ws_tasks.lock().await;
    for (_, handle) in tasks.drain() {
        handle.abort();
    }
}

/// Handle an incoming WebSocket command from the client.
async fn handle_command(
    cmd: WsCommand,
    ws_tasks: &Arc<tokio::sync::Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
    ws_ticker_tx: &broadcast::Sender<WsTickerUpdate>,
) {
    match cmd.command.as_str() {
        "start_ws_stream" => {
            let provider_id = match cmd.provider_id {
                Some(id) => id,
                None => return,
            };
            let symbols = cmd.symbols.unwrap_or_default();
            if symbols.is_empty() {
                return;
            }

            // Stop existing stream for this provider if any
            {
                let mut tasks = ws_tasks.lock().await;
                if let Some(handle) = tasks.remove(&provider_id) {
                    handle.abort();
                }
            }

            // Create WS provider and start streaming
            let ws_provider = match create_ws_provider(&provider_id) {
                Some(p) => p,
                None => return,
            };

            let sender = Arc::new(ws_ticker_tx.clone());
            if let Ok(handle) = ws_provider.subscribe(symbols, sender).await {
                let mut tasks = ws_tasks.lock().await;
                tasks.insert(provider_id, handle);
            }
        }
        "stop_ws_stream" => {
            let provider_id = match cmd.provider_id {
                Some(id) => id,
                None => return,
            };
            let mut tasks = ws_tasks.lock().await;
            if let Some(handle) = tasks.remove(&provider_id) {
                handle.abort();
            }
        }
        _ => {}
    }
}

// ─── REST Handlers for WS Stream Control ────────────────────────────────────────

/// Request body for `POST /ws/stream/start` and `POST /ws/stream/stop`.
#[derive(Debug, Deserialize)]
struct WsStreamRequest {
    provider_id: String,
    #[serde(default)]
    symbols: Option<Vec<String>>,
}

/// POST /ws/stream/start — start a WebSocket stream for a provider.
///
/// In server mode, WebSocket streams are managed per-connection via the `/api/ws` endpoint.
/// This REST endpoint is provided for API compatibility. It returns success to acknowledge
/// the request — the actual streaming happens through the WebSocket connection.
async fn start_ws_stream(
    State(_state): State<Arc<CoreState>>,
    axum::extract::Json(body): axum::extract::Json<WsStreamRequest>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    use crate::api::{ApiError, ApiResponse};

    let symbols = body.symbols.unwrap_or_default();
    if symbols.is_empty() {
        return ApiError::bad_request("symbols must not be empty").into_response();
    }

    // Validate provider supports WS
    if create_ws_provider(&body.provider_id).is_none() {
        return ApiError::bad_request(format!(
            "Provider '{}' does not support WebSocket streaming",
            body.provider_id
        ))
        .into_response();
    }

    // In server mode, WS streams are managed per WebSocket connection.
    // Return success — the client should send start_ws_stream via the WS connection.
    ApiResponse::ok(serde_json::json!({
        "success": true,
        "message": "Send start_ws_stream command via WebSocket connection for real-time data"
    })).into_response()
}

/// POST /ws/stream/stop — stop a WebSocket stream for a provider.
async fn stop_ws_stream(
    State(_state): State<Arc<CoreState>>,
    axum::extract::Json(body): axum::extract::Json<WsStreamRequest>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    use crate::api::ApiResponse;

    // In server mode, WS streams are managed per WebSocket connection.
    // Return success — the client should send stop_ws_stream via the WS connection.
    let _ = &body.provider_id;
    ApiResponse::ok(serde_json::json!({
        "success": true,
        "message": "Send stop_ws_stream command via WebSocket connection"
    })).into_response()
}
