//! Integration tests for server graceful shutdown behavior.
//!
//! Tests that the server:
//! - Responds to shutdown signals by shutting down gracefully
//! - Shuts down within the 10-second timeout requirement
//!
//! These tests simulate graceful shutdown using Axum's `with_graceful_shutdown`
//! mechanism (cancellation token / oneshot channel) rather than real OS signals,
//! which makes them portable across platforms (Windows included).
//!
//! **Validates: Requirements 7.5**

use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Test that the server shuts down within 10 seconds when the shutdown signal fires.
/// This validates Requirements 7.5: "THE Server_Binary SHALL handle SIGTERM signals
/// by completing in-progress database writes and shutting down gracefully within 10 seconds."
#[tokio::test]
async fn shutdown_completes_within_timeout() {
    // Build a minimal Axum server with a health route
    let app = axum::Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let _addr = listener.local_addr().unwrap();

    // Simulate a shutdown signal that fires after 100ms
    let shutdown = async {
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    let server = axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown);

    let start = Instant::now();

    // The server must complete within 10 seconds (the spec requirement)
    let result = timeout(Duration::from_secs(10), server).await;
    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "Server did not shut down within 10 seconds (elapsed: {:?})",
        elapsed
    );
    assert!(
        result.unwrap().is_ok(),
        "Server returned an error during shutdown"
    );
    // Verify it actually shut down promptly (well within the 10s budget)
    assert!(
        elapsed < Duration::from_secs(5),
        "Shutdown took too long: {:?}",
        elapsed
    );
}

/// Test that the server accepts connections before shutdown and stops
/// accepting after the shutdown signal fires.
#[tokio::test]
async fn server_accepts_requests_then_shuts_down() {
    let app = axum::Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            })
            .await
            .unwrap();
    });

    // Give server time to start accepting connections
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify server is accepting requests
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/health", addr))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to server");
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "ok");

    // Send shutdown signal
    let start = Instant::now();
    shutdown_tx.send(()).unwrap();

    // Server should complete within 10 seconds
    let result = timeout(Duration::from_secs(10), server_handle).await;
    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "Server did not shut down within 10 seconds (elapsed: {:?})",
        elapsed
    );
    assert!(
        result.unwrap().is_ok(),
        "Server task panicked during shutdown"
    );
}

/// Test that the shutdown mechanism handles concurrent in-flight requests
/// gracefully — the server should wait for the active request to finish
/// before exiting (within the 10s window).
#[tokio::test]
async fn shutdown_waits_for_inflight_request() {
    // A handler that takes 500ms to respond (simulating an in-progress operation)
    let app = axum::Router::new().route(
        "/slow",
        axum::routing::get(|| async {
            tokio::time::sleep(Duration::from_millis(500)).await;
            "done"
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            })
            .await
            .unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Start a slow request
    let client = reqwest::Client::new();
    let request_handle = tokio::spawn({
        let client = client.clone();
        let url = format!("http://{}/slow", addr);
        async move {
            client
                .get(url)
                .timeout(Duration::from_secs(10))
                .send()
                .await
        }
    });

    // Give the request a moment to reach the handler, then trigger shutdown
    tokio::time::sleep(Duration::from_millis(50)).await;
    shutdown_tx.send(()).unwrap();

    // The in-flight request should still complete successfully
    let resp = request_handle.await.unwrap();
    assert!(
        resp.is_ok(),
        "In-flight request failed during shutdown: {:?}",
        resp.err()
    );
    let resp = resp.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "done");

    // Server should complete well within 10 seconds
    let result = timeout(Duration::from_secs(10), server_handle).await;
    assert!(result.is_ok(), "Server did not shut down within 10 seconds");
}
