//! Property-based test: WebSocket broadcast to all clients.
//!
//! **Feature: web-server-mode, Property 4: WebSocket broadcast to all clients**
//!
//! *For any* N connected WebSocket clients (N ≥ 1), when an event is published to the event bus,
//! all N clients SHALL receive the event message independently.
//!
//! **Validates: Requirements 4.6**

use std::sync::Arc;

use futures::StreamExt;
use proptest::prelude::*;
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 4.6**
    ///
    /// For any N connected WebSocket clients (1..=8), when an event is
    /// published to the event bus, all N clients shall receive the event independently.
    #[test]
    fn ws_broadcast_reaches_all_clients(
        n_clients in 1u8..=8u8,
        provider_id in "[a-z]{3,10}",
        error_msg in "[a-zA-Z0-9 ]{5,30}",
        symbol in "[A-Z]{2,6}",
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tmp = tempfile::TempDir::new().unwrap();
            let state = stockenboard_lib::core_state::CoreState::new(tmp.path()).unwrap();
            let event_bus = state.event_bus.clone();
            let app = stockenboard_lib::api::build_router(Arc::new(state));

            // Bind to a random available port
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();

            // Spawn the server
            let server_handle = tokio::spawn(async move {
                axum::serve(listener, app).await.ok();
            });

            // Connect N WebSocket clients
            let n = n_clients as usize;
            let ws_url = format!("ws://127.0.0.1:{}/api/ws", addr.port());
            let mut clients = Vec::with_capacity(n);

            for _ in 0..n {
                let (ws_stream, _) = connect_async(&ws_url).await
                    .expect("Failed to connect WebSocket client");
                clients.push(ws_stream);
            }

            // Give clients a moment to subscribe to the event bus
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            // Emit an event to the event bus
            let test_event = stockenboard_lib::events::AppEvent::PriceError {
                provider_id: provider_id.clone(),
                symbols: vec![symbol.clone()],
                error: error_msg.clone(),
            };
            event_bus.send(test_event).unwrap();

            // Verify all N clients receive the event
            for (i, client) in clients.iter_mut().enumerate() {
                let msg = tokio::time::timeout(
                    tokio::time::Duration::from_secs(5),
                    client.next(),
                )
                .await
                .unwrap_or_else(|_| panic!("Client {} timed out waiting for message", i))
                .unwrap_or_else(|| panic!("Client {} stream ended unexpectedly", i))
                .unwrap_or_else(|e| panic!("Client {} got error: {}", i, e));

                match msg {
                    Message::Text(text) => {
                        let json: serde_json::Value = serde_json::from_str(&text)
                            .expect("Message should be valid JSON");

                        // Verify envelope structure
                        assert!(
                            json.get("type").is_some(),
                            "Client {} message missing 'type' field: {:?}", i, json
                        );
                        assert!(
                            json.get("data").is_some(),
                            "Client {} message missing 'data' field: {:?}", i, json
                        );
                        assert!(
                            json.get("timestamp").is_some(),
                            "Client {} message missing 'timestamp' field: {:?}", i, json
                        );

                        // Verify it's the correct event type
                        assert_eq!(
                            json["type"].as_str().unwrap(),
                            "price-error",
                            "Client {} got wrong event type", i
                        );

                        // Verify data content matches what was sent.
                        // The WS serialization produces a HashMap: { "provider:symbol": "error msg" }
                        let data = &json["data"];
                        let expected_key = format!("{}:{}", provider_id, symbol);
                        assert!(
                            data.get(&expected_key).is_some(),
                            "Client {} data missing key '{}': {:?}", i, expected_key, data
                        );
                        assert_eq!(
                            data[&expected_key].as_str().unwrap(),
                            error_msg.as_str(),
                            "Client {} got wrong error message for key '{}'", i, expected_key
                        );
                    }
                    other => {
                        panic!(
                            "Client {} received non-text message: {:?}", i, other
                        );
                    }
                }
            }

            // Cleanup: abort server
            server_handle.abort();
        });
    }
}
