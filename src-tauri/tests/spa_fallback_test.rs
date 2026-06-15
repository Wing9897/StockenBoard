//! Property-based test: SPA fallback for non-API, non-file paths.
//!
//! **Property 7: SPA fallback for non-API, non-file paths**
//! *For any* request path that does not match an API route (`/api/...`) and does not
//! match an existing static file, the server SHALL respond with the contents of
//! `index.html` and status 200.
//!
//! **Validates: Requirements 6.2**

use std::sync::Arc;

use axum::body::Body;
use http::Request;
use http_body_util::BodyExt;
use proptest::prelude::*;
use tower::ServiceExt;

/// Known content for the test index.html — unique enough to verify SPA fallback.
const INDEX_HTML_CONTENT: &str = "<!DOCTYPE html><html><body>SPA Fallback</body></html>";

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// **Validates: Requirements 6.2**
    ///
    /// Generate random paths that do NOT start with `/api/` and do NOT match any
    /// static file in the temp directory, then verify the server returns HTTP 200
    /// with the index.html content (SPA fallback).
    #[test]
    fn spa_fallback_serves_index_html_for_unknown_paths(
        // Generate 1-4 path segments of lowercase letters (e.g., /about, /dash/settings, /foo/bar/baz)
        segments in prop::collection::vec("[a-z]{2,12}", 1..=4)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Set up a temp directory with just index.html
            let tmp = tempfile::TempDir::new().unwrap();
            std::fs::write(tmp.path().join("index.html"), INDEX_HTML_CONTENT).unwrap();

            // Build the full router with static file serving
            let state = stockenboard_lib::core_state::CoreState::new(tmp.path()).unwrap();
            let app = stockenboard_lib::api::build_router_with_static(
                Arc::new(state),
                tmp.path(),
            );

            // Build the path from segments: /seg1/seg2/...
            let path = format!("/{}", segments.join("/"));

            // Sanity: our generated path must NOT start with /api/
            assert!(
                !path.starts_with("/api/"),
                "Generated path unexpectedly starts with /api/: {}",
                path
            );

            let response = app
                .oneshot(
                    Request::builder()
                        .uri(&path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            // SPA fallback should return 200
            assert_eq!(
                response.status(),
                http::StatusCode::OK,
                "Expected 200 for SPA fallback on path: {}, got: {}",
                path,
                response.status()
            );

            // Body should contain the index.html content
            let body = response.into_body().collect().await.unwrap().to_bytes();
            let body_str = String::from_utf8_lossy(&body);
            assert!(
                body_str.contains("SPA Fallback"),
                "Expected index.html content for path: {}, got: {}",
                path,
                body_str
            );
        });
    }

    /// **Validates: Requirements 6.2**
    ///
    /// Generate random deep paths (with numbers and mixed segments) that don't match
    /// API routes or files, verify SPA fallback still works.
    #[test]
    fn spa_fallback_works_for_paths_with_numbers_and_special_segments(
        prefix in "[a-z]{2,8}",
        id in "[a-z0-9]{1,10}",
        suffix in "[a-z]{2,8}"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tmp = tempfile::TempDir::new().unwrap();
            std::fs::write(tmp.path().join("index.html"), INDEX_HTML_CONTENT).unwrap();

            let state = stockenboard_lib::core_state::CoreState::new(tmp.path()).unwrap();
            let app = stockenboard_lib::api::build_router_with_static(
                Arc::new(state),
                tmp.path(),
            );

            // Build path like /dashboard/abc123/settings
            let path = format!("/{}/{}/{}", prefix, id, suffix);

            // Must not start with /api/ (given our generators, this won't happen,
            // but prop_assume for safety)
            assert!(!path.starts_with("/api/"));

            let response = app
                .oneshot(
                    Request::builder()
                        .uri(&path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                http::StatusCode::OK,
                "Expected 200 for SPA fallback on path: {}, got: {}",
                path,
                response.status()
            );

            let body = response.into_body().collect().await.unwrap().to_bytes();
            let body_str = String::from_utf8_lossy(&body);
            assert!(
                body_str.contains("SPA Fallback"),
                "Expected index.html content for path: {}, got: {}",
                path,
                body_str
            );
        });
    }
}
