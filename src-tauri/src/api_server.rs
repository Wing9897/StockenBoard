/// StockenBoard HTTP API Server
/// 提供簡單的 REST API 讓外部程式（如 AI）訪問實時和歷史數據
use crate::commands::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

// ── 數據結構 ──

#[derive(Debug, Clone, Serialize)]
pub struct ApiPrice {
    pub symbol: String,
    pub provider: String,
    pub price: f64,
    pub change_24h: Option<f64>,
    pub volume: Option<f64>,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiSubscription {
    pub id: i64,
    pub sub_type: String,
    pub symbol: String,
    pub display_name: Option<String>,
    pub provider: String,
    pub asset_type: String,
    pub recording_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub symbol: Option<String>,
    pub provider: Option<String>,
    pub subscription_id: Option<i64>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    1000
}

// ── API Handlers ──

/// GET /api/prices - 獲取所有最新價格（從內存 cache）
async fn get_prices(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cache = state.polling.cache.read().await;
    let prices: Vec<ApiPrice> = cache
        .iter()
        .map(|(key, data)| {
            let parts: Vec<&str> = key.split(':').collect();
            let provider = parts.first().unwrap_or(&"unknown").to_string();
            let symbol = parts.get(1..).unwrap_or(&[]).join(":");
            ApiPrice {
                symbol,
                provider,
                price: data.price,
                change_24h: data.change_percent_24h,
                volume: data.volume,
                timestamp: chrono::Utc::now().timestamp(),
                extra: data
                    .extra
                    .as_ref()
                    .map(|m| serde_json::to_value(m).unwrap_or(serde_json::Value::Null)),
            }
        })
        .collect();

    Json(serde_json::json!({
        "prices": prices,
        "count": prices.len(),
        "timestamp": chrono::Utc::now().timestamp()
    }))
}

/// GET /api/prices/:provider/:symbol - 獲取特定價格
async fn get_price_by_key(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((provider, symbol)): axum::extract::Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let cache = state.polling.cache.read().await;
    let key = format!("{}:{}", provider, symbol);

    match cache.get(&key) {
        Some(data) => Ok(Json(serde_json::json!({
            "symbol": symbol,
            "provider": provider,
            "price": data.price,
            "change_24h": data.change_percent_24h,
            "volume": data.volume,
            "timestamp": chrono::Utc::now().timestamp(),
            "extra": data.extra.as_ref().map(|m| {
                serde_json::to_value(m).unwrap_or(serde_json::Value::Null)
            })
        }))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// GET /api/history - 查詢歷史數據（從 SQL）
async fn get_history(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let db_path = state.db_path.read().unwrap().clone().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "DB path not set".to_string(),
    ))?;

    let conn = Connection::open(&db_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 構建查詢
    let mut sql = String::from(
        "SELECT ph.price, ph.change_pct, ph.volume, ph.pre_price, ph.post_price, ph.recorded_at, 
                s.symbol, s.selected_provider_id, s.sub_type
         FROM price_history ph
         JOIN subscriptions s ON ph.subscription_id = s.id
         WHERE 1=1",
    );
    let mut conditions = Vec::new();

    if let Some(sub_id) = params.subscription_id {
        sql.push_str(" AND ph.subscription_id = ?");
        conditions.push(sub_id.to_string());
    }
    if let Some(ref symbol) = params.symbol {
        sql.push_str(" AND s.symbol = ?");
        conditions.push(symbol.clone());
    }
    if let Some(ref provider) = params.provider {
        sql.push_str(" AND s.selected_provider_id = ?");
        conditions.push(provider.clone());
    }
    if let Some(from) = params.from {
        sql.push_str(" AND ph.recorded_at >= ?");
        conditions.push(from.to_string());
    }
    if let Some(to) = params.to {
        sql.push_str(" AND ph.recorded_at <= ?");
        conditions.push(to.to_string());
    }

    sql.push_str(" ORDER BY ph.recorded_at DESC LIMIT ?");
    conditions.push(params.limit.to_string());

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let params_refs: Vec<&dyn rusqlite::ToSql> = conditions
        .iter()
        .map(|s| s as &dyn rusqlite::ToSql)
        .collect();

    let records: Result<Vec<_>, _> = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(serde_json::json!({
                "price": row.get::<_, f64>(0)?,
                "change_pct": row.get::<_, Option<f64>>(1)?,
                "volume": row.get::<_, Option<f64>>(2)?,
                "pre_price": row.get::<_, Option<f64>>(3)?,
                "post_price": row.get::<_, Option<f64>>(4)?,
                "recorded_at": row.get::<_, i64>(5)?,
                "symbol": row.get::<_, String>(6)?,
                "provider": row.get::<_, String>(7)?,
                "type": row.get::<_, String>(8)?
            }))
        })
        .and_then(|rows| rows.collect());

    let records = records.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "records": records,
        "count": records.len(),
        "query": {
            "symbol": params.symbol,
            "provider": params.provider,
            "from": params.from,
            "to": params.to,
            "limit": params.limit
        }
    })))
}

/// GET /api/subscriptions - 獲取所有訂閱
async fn get_subscriptions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let db_path = state.db_path.read().unwrap().clone().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "DB path not set".to_string(),
    ))?;

    let conn = Connection::open(&db_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, sub_type, symbol, display_name, selected_provider_id, asset_type, record_enabled 
             FROM subscriptions 
             ORDER BY sort_order, id"
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let subs: Result<Vec<_>, _> = stmt
        .query_map([], |row| {
            Ok(ApiSubscription {
                id: row.get(0)?,
                sub_type: row.get(1)?,
                symbol: row.get(2)?,
                display_name: row.get(3)?,
                provider: row.get(4)?,
                asset_type: row.get(5)?,
                recording_enabled: row.get::<_, i64>(6)? != 0,
            })
        })
        .and_then(|rows| rows.collect());

    let subs = subs.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "subscriptions": subs,
        "count": subs.len()
    })))
}

/// GET /api/status - 系統狀態
async fn get_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cache = state.polling.cache.read().await;
    let ticks = state.polling.ticks.read().await;
    let is_unattended = state.polling.is_unattended().await;

    let tick_info: HashMap<String, i64> = ticks
        .iter()
        .map(|(k, v)| (k.clone(), v.fetched_at))
        .collect();

    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running",
        "unattended_mode": is_unattended,
        "cache_size": cache.len(),
        "active_providers": ticks.len(),
        "last_poll_ticks": tick_info,
        "timestamp": chrono::Utc::now().timestamp()
    }))
}

// ── Server ──

pub async fn start_api_server(
    state: Arc<AppState>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/api/prices", get(get_prices))
        .route("/api/prices/:provider/:symbol", get(get_price_by_key))
        .route("/api/history", get(get_history))
        .route("/api/subscriptions", get(get_subscriptions))
        .route("/api/status", get(get_status))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port);
    eprintln!("[API] Starting HTTP server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
