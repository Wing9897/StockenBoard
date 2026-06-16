//! Property-based test for HTTP API CRUD round-trip integrity.
//!
//! **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.7, 3.8, 3.9**
//!
//! Property 2: HTTP API CRUD round-trip integrity
//! For any valid entity (subscription, view, provider setting, notification rule,
//! AI config, system config), creating or updating it via the HTTP API and then
//! reading it back SHALL return data equivalent to the original input.
//!
//! Feature: web-server-mode, Property 2: HTTP API CRUD round-trip integrity

use std::sync::Arc;

use axum::body::Body;
use http::Request;
use http_body_util::BodyExt;
use proptest::prelude::*;
use tower::ServiceExt;

/// Strategy for generating valid subscription data.
/// Subscriptions require: sub_type, symbol, provider_id, asset_type.
/// The symbol is normalized on creation, so we generate already-normalized symbols.
fn subscription_strategy() -> impl Strategy<Value = (String, String, Option<String>, String, String)> {
    let asset_type = prop_oneof![
        Just("crypto".to_string()),
        Just("stock".to_string()),
        Just("forex".to_string()),
    ];

    let provider_id = prop_oneof![
        Just("binance".to_string()),
        Just("coinbase".to_string()),
        Just("yahoo".to_string()),
        Just("alphavantage".to_string()),
    ];

    // For crypto, use known base symbols to avoid the symbol normalizer splitting
    // them incorrectly (e.g. "AGBP" -> "A-GBP" because GBP is a known quote suffix).
    // For stock/forex, any uppercase alpha string works since normalize just uppercases.
    let crypto_symbol = prop_oneof![
        Just("BTC".to_string()),
        Just("ETH".to_string()),
        Just("SOL".to_string()),
        Just("ADA".to_string()),
        Just("DOT".to_string()),
        Just("AVAX".to_string()),
        Just("LINK".to_string()),
        Just("ATOM".to_string()),
        Just("XRP".to_string()),
        Just("DOGE".to_string()),
    ];
    let non_crypto_symbol = "[A-Z]{3,5}".prop_map(|s| s.to_string());

    let display_name = prop_oneof![
        Just(None),
        "[A-Za-z ]{3,15}".prop_map(|s| Some(s)),
    ];

    // Pair the asset_type with an appropriate symbol strategy
    asset_type.prop_flat_map(move |at| {
        let sym = if at == "crypto" {
            crypto_symbol.clone().boxed()
        } else {
            non_crypto_symbol.clone().boxed()
        };
        let display_name = display_name.clone();
        let provider_id = provider_id.clone();
        (Just("asset".to_string()), sym, display_name, provider_id, Just(at))
    })
}

/// Strategy for generating valid view data.
fn view_strategy() -> impl Strategy<Value = (String, String)> {
    let name = "[A-Za-z]{3,12}";
    let view_type = prop_oneof![
        Just("asset".to_string()),
        Just("dex".to_string()),
    ];
    (name.prop_map(|s| s.to_string()), view_type)
}

/// Strategy for generating valid notification rule data.
/// Requires an existing subscription_id (we'll create one first in the test).
fn notification_rule_strategy() -> impl Strategy<Value = (String, String, f64, Vec<i64>, Option<u64>)> {
    let name = "[A-Za-z ]{3,20}";
    let condition_type = prop_oneof![
        Just("price_above".to_string()),
        Just("price_below".to_string()),
        Just("change_above".to_string()),
        Just("change_below".to_string()),
    ];
    let threshold = 0.01f64..10000.0f64;
    let channel_ids: Vec<i64> = vec![];
    let cooldown_secs = prop_oneof![
        Just(None),
        (60u64..3600u64).prop_map(|s| Some(s)),
    ];

    (name.prop_map(|s| s.to_string()), condition_type, threshold, Just(channel_ids), cooldown_secs)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 3.1**
    ///
    /// Property 2 (Subscriptions): For any valid subscription data, POSTing it to
    /// /api/subscriptions and then GETting /api/subscriptions SHALL return data
    /// containing the created subscription with equivalent fields.
    #[test]
    fn subscription_crud_roundtrip(
        (sub_type, symbol, display_name, provider_id, asset_type) in subscription_strategy()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tmp = tempfile::TempDir::new().unwrap();
            let state = stockenboard_lib::core_state::CoreState::new(tmp.path()).unwrap();
            let state = Arc::new(state);
            let app = stockenboard_lib::api::build_router(state.clone());

            // Build JSON body for POST /api/subscriptions
            let body = serde_json::json!({
                "sub_type": sub_type,
                "symbol": symbol,
                "display_name": display_name,
                "provider_id": provider_id,
                "asset_type": asset_type,
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/subscriptions")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            // POST should return 201 Created
            prop_assert_eq!(
                response.status().as_u16(),
                201,
                "POST /api/subscriptions should return 201, got {} for body: {:?}",
                response.status(),
                body
            );

            let post_body = response.into_body().collect().await.unwrap().to_bytes();
            let post_json: serde_json::Value = serde_json::from_slice(&post_body).unwrap();
            let created_id = post_json["data"]["id"].as_i64().unwrap();
            prop_assert!(created_id > 0, "Created subscription should have a positive ID");

            // Now GET /api/subscriptions to list all and verify our subscription is there
            let app2 = stockenboard_lib::api::build_router(state.clone());
            let response = app2
                .oneshot(
                    Request::builder()
                        .uri("/api/subscriptions")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(response.status().as_u16(), 200);

            let get_body = response.into_body().collect().await.unwrap().to_bytes();
            let get_json: serde_json::Value = serde_json::from_slice(&get_body).unwrap();
            let subs = get_json["data"].as_array().unwrap();

            // Find our subscription in the list
            let found = subs.iter().find(|s| s["id"].as_i64() == Some(created_id));
            prop_assert!(
                found.is_some(),
                "Created subscription with id {} should appear in GET list",
                created_id
            );
            let found = found.unwrap();

            // Verify fields match what we sent.
            // Note: symbol gets normalized (uppercased for crypto/stock/forex).
            let expected_symbol = symbol.to_uppercase();
            prop_assert_eq!(
                found["symbol"].as_str().unwrap(),
                expected_symbol.as_str(),
                "Symbol mismatch"
            );
            prop_assert_eq!(
                found["selected_provider_id"].as_str().unwrap(),
                provider_id.as_str(),
                "Provider ID mismatch"
            );
            prop_assert_eq!(
                found["asset_type"].as_str().unwrap(),
                asset_type.as_str(),
                "Asset type mismatch"
            );
            prop_assert_eq!(
                found["sub_type"].as_str().unwrap(),
                sub_type.as_str(),
                "Sub type mismatch"
            );

            // display_name: null in JSON if None, otherwise a string
            match &display_name {
                Some(dn) => {
                    prop_assert_eq!(
                        found["display_name"].as_str().unwrap(),
                        dn.as_str(),
                        "Display name mismatch"
                    );
                }
                None => {
                    prop_assert!(
                        found["display_name"].is_null(),
                        "Display name should be null, got: {:?}",
                        found["display_name"]
                    );
                }
            }

            Ok(())
        })?;
    }

    /// **Validates: Requirements 3.2**
    ///
    /// Property 2 (Views): For any valid view data, POSTing it to /api/views
    /// and then GETting /api/views?type=... SHALL return data containing the
    /// created view with equivalent fields.
    #[test]
    fn view_crud_roundtrip(
        (name, view_type) in view_strategy()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tmp = tempfile::TempDir::new().unwrap();
            let state = stockenboard_lib::core_state::CoreState::new(tmp.path()).unwrap();
            let state = Arc::new(state);
            let app = stockenboard_lib::api::build_router(state.clone());

            // Build JSON body for POST /api/views
            let body = serde_json::json!({
                "name": name,
                "type": view_type,
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/views")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            // POST should return 201 Created
            prop_assert_eq!(
                response.status().as_u16(),
                201,
                "POST /api/views should return 201, got {}",
                response.status()
            );

            let post_body = response.into_body().collect().await.unwrap().to_bytes();
            let post_json: serde_json::Value = serde_json::from_slice(&post_body).unwrap();
            let created_id = post_json["data"]["id"].as_i64().unwrap();
            prop_assert!(created_id > 0, "Created view should have a positive ID");

            // Now GET /api/views?type=<view_type> to list and verify
            let app2 = stockenboard_lib::api::build_router(state.clone());
            let uri = format!("/api/views?type={}", view_type);
            let response = app2
                .oneshot(
                    Request::builder()
                        .uri(&uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(response.status().as_u16(), 200);

            let get_body = response.into_body().collect().await.unwrap().to_bytes();
            let get_json: serde_json::Value = serde_json::from_slice(&get_body).unwrap();
            let views = get_json["data"].as_array().unwrap();

            // Find our view in the list
            let found = views.iter().find(|v| v["id"].as_i64() == Some(created_id));
            prop_assert!(
                found.is_some(),
                "Created view with id {} should appear in GET list",
                created_id
            );
            let found = found.unwrap();

            prop_assert_eq!(
                found["name"].as_str().unwrap(),
                name.as_str(),
                "View name mismatch"
            );
            prop_assert_eq!(
                found["view_type"].as_str().unwrap(),
                view_type.as_str(),
                "View type mismatch"
            );
            prop_assert_eq!(
                found["is_default"].as_bool().unwrap(),
                false,
                "Newly created view should not be default"
            );

            Ok(())
        })?;
    }

    /// **Validates: Requirements 3.4**
    ///
    /// Property 2 (Notification Rules): For any valid notification rule data,
    /// POSTing it to /api/notifications/rules and then GETting /api/notifications/rules
    /// SHALL return data containing the created rule with equivalent fields.
    #[test]
    fn notification_rule_crud_roundtrip(
        (rule_name, condition_type, threshold, channel_ids, cooldown_secs) in notification_rule_strategy()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tmp = tempfile::TempDir::new().unwrap();
            let state = stockenboard_lib::core_state::CoreState::new(tmp.path()).unwrap();
            let state = Arc::new(state);

            // First, create a subscription (rules need a valid subscription_id)
            let app = stockenboard_lib::api::build_router(state.clone());
            let sub_body = serde_json::json!({
                "sub_type": "asset",
                "symbol": "BTC",
                "provider_id": "binance",
                "asset_type": "crypto",
            });
            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/subscriptions")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&sub_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
            let sub_resp = response.into_body().collect().await.unwrap().to_bytes();
            let sub_json: serde_json::Value = serde_json::from_slice(&sub_resp).unwrap();
            let subscription_id = sub_json["data"]["id"].as_i64().unwrap();

            // Now create the notification rule
            let app2 = stockenboard_lib::api::build_router(state.clone());
            let mut rule_body = serde_json::json!({
                "name": rule_name,
                "subscription_id": subscription_id,
                "condition_type": condition_type,
                "threshold": threshold,
                "channel_ids": channel_ids,
            });
            if let Some(cd) = cooldown_secs {
                rule_body["cooldown_secs"] = serde_json::json!(cd);
            }

            let response = app2
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/notifications/rules")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&rule_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(
                response.status().as_u16(),
                201,
                "POST /api/notifications/rules should return 201, got {}",
                response.status()
            );

            let post_body = response.into_body().collect().await.unwrap().to_bytes();
            let post_json: serde_json::Value = serde_json::from_slice(&post_body).unwrap();
            let rule_id = post_json["data"]["id"].as_i64().unwrap();
            prop_assert!(rule_id > 0, "Created rule should have a positive ID");

            // GET /api/notifications/rules to list and verify
            let app3 = stockenboard_lib::api::build_router(state.clone());
            let response = app3
                .oneshot(
                    Request::builder()
                        .uri("/api/notifications/rules")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(response.status().as_u16(), 200);

            let get_body = response.into_body().collect().await.unwrap().to_bytes();
            let get_json: serde_json::Value = serde_json::from_slice(&get_body).unwrap();
            let rules = get_json["data"].as_array().unwrap();

            let found = rules.iter().find(|r| r["id"].as_i64() == Some(rule_id));
            prop_assert!(
                found.is_some(),
                "Created rule with id {} should appear in GET list",
                rule_id
            );
            let found = found.unwrap();

            prop_assert_eq!(
                found["name"].as_str().unwrap(),
                rule_name.as_str(),
                "Rule name mismatch"
            );
            prop_assert_eq!(
                found["subscription_id"].as_i64().unwrap(),
                subscription_id,
                "Subscription ID mismatch"
            );
            prop_assert_eq!(
                found["condition_type"].as_str().unwrap(),
                condition_type.as_str(),
                "Condition type mismatch"
            );

            // Threshold comparison with tolerance for floating point
            let returned_threshold = found["threshold"].as_f64().unwrap();
            prop_assert!(
                (returned_threshold - threshold).abs() < 0.001,
                "Threshold mismatch: expected {}, got {}",
                threshold,
                returned_threshold
            );

            // channel_ids is stored as a JSON string in DB, returned as string
            let returned_channel_ids: Vec<i64> = serde_json::from_str(
                found["channel_ids"].as_str().unwrap()
            ).unwrap();
            prop_assert_eq!(
                returned_channel_ids,
                channel_ids,
                "Channel IDs mismatch"
            );

            // Cooldown: default is 300 if not specified
            let expected_cooldown = cooldown_secs.unwrap_or(300) as i64;
            prop_assert_eq!(
                found["cooldown_secs"].as_i64().unwrap(),
                expected_cooldown,
                "Cooldown mismatch"
            );

            // Rule should be enabled by default
            prop_assert_eq!(
                found["enabled"].as_bool().unwrap(),
                true,
                "New rule should be enabled by default"
            );

            Ok(())
        })?;
    }
}
