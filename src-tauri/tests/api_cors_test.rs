//! Property-based test: CORS permissive headers.
//!
//! **Property 8: CORS permissive headers**
//! *For any* HTTP request containing an `Origin` header, the response SHALL include
//! permissive CORS headers (`Access-Control-Allow-Origin: *` or reflecting the request origin).
//!
//! **Validates: Requirements 9.3**

use std::sync::Arc;

use axum::body::Body;
use http::Request;
use proptest::prelude::*;
use tower::ServiceExt;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// For any random Origin header value, a CORS preflight (OPTIONS) request
    /// to the API must return the `Access-Control-Allow-Origin` header.
    #[test]
    fn cors_allows_any_origin(
        origin in "https?://[a-z]{3,10}\\.[a-z]{2,5}(:[0-9]{2,5})?"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tmp = tempfile::TempDir::new().unwrap();
            let state = stockenboard_lib::core_state::CoreState::new(tmp.path()).unwrap();
            let app = stockenboard_lib::api::build_router(Arc::new(state));

            let response = app
                .oneshot(
                    Request::builder()
                        .method("OPTIONS")
                        .uri("/api/subscriptions")
                        .header("Origin", &origin)
                        .header("Access-Control-Request-Method", "GET")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            // CORS permissive should always return Access-Control-Allow-Origin
            let acao = response.headers().get("access-control-allow-origin");
            prop_assert!(
                acao.is_some(),
                "Missing Access-Control-Allow-Origin header for origin: {}",
                origin
            );

            Ok(())
        })?;
    }

    /// For any random Origin header value, a regular GET request to the API
    /// must also return the `Access-Control-Allow-Origin` header in the response.
    #[test]
    fn cors_headers_on_regular_requests(
        origin in "https?://[a-z]{3,10}\\.[a-z]{2,5}(:[0-9]{2,5})?"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tmp = tempfile::TempDir::new().unwrap();
            let state = stockenboard_lib::core_state::CoreState::new(tmp.path()).unwrap();
            let app = stockenboard_lib::api::build_router(Arc::new(state));

            let response = app
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/api/subscriptions")
                        .header("Origin", &origin)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            // CORS permissive should always return Access-Control-Allow-Origin
            let acao = response.headers().get("access-control-allow-origin");
            prop_assert!(
                acao.is_some(),
                "Missing Access-Control-Allow-Origin header on GET for origin: {}",
                origin
            );

            Ok(())
        })?;
    }
}
