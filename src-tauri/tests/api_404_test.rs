//! Property-based test for unknown API path returning 404 JSON.
//!
//! **Validates: Requirements 3.13**
//!
//! Property 3: Unknown path returns 404 JSON
//! For any request path that does not match a defined API route,
//! the HTTP API SHALL return HTTP 404 with a JSON error body
//! containing `code` and `message` fields.

use std::sync::Arc;

use axum::body::Body;
use http::Request;
use http_body_util::BodyExt;
use proptest::prelude::*;
use tower::ServiceExt;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// **Validates: Requirements 3.13**
    ///
    /// Generate random path segments that don't match any known API route,
    /// verify the server returns 404 JSON with `code` and `message` fields.
    #[test]
    fn unknown_api_path_returns_404_json(
        path_segment in "[a-z]{3,15}"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tmp = tempfile::TempDir::new().unwrap();
            let state = stockenboard_lib::core_state::CoreState::new(tmp.path()).unwrap();
            let app = stockenboard_lib::api::build_router(Arc::new(state));

            // Prefix with "nonexistent_" to guarantee we never accidentally hit a real route
            let path = format!("/api/nonexistent_{}", path_segment);

            let response = app
                .oneshot(
                    Request::builder()
                        .uri(&path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(response.status(), http::StatusCode::NOT_FOUND);

            let body = response.into_body().collect().await.unwrap().to_bytes();
            let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

            prop_assert!(
                json.get("error").is_some(),
                "Response should have 'error' field, got: {:?}", json
            );
            let error = json.get("error").unwrap();
            prop_assert!(
                error.get("code").is_some(),
                "Error should have 'code' field, got: {:?}", error
            );
            prop_assert!(
                error.get("message").is_some(),
                "Error should have 'message' field, got: {:?}", error
            );
            prop_assert_eq!(
                error["code"].as_str().unwrap(),
                "not_found"
            );

            Ok(())
        })?;
    }
}
