//! Property-based test: Static file serving with correct MIME and cache headers.
//!
//! **Property 6: Static file serving with correct MIME and cache headers**
//! *For any* static file request, the server SHALL return the file content with the correct
//! MIME type based on extension, and set `Cache-Control: public, max-age=31536000, immutable`
//! for hashed assets or `Cache-Control: no-cache, no-store, must-revalidate` for `index.html`.
//!
//! **Validates: Requirements 6.1, 6.3, 6.4**

use std::sync::Arc;

use axum::body::Body;
use http::Request;
use proptest::prelude::*;
use tower::ServiceExt;

/// Returns the expected MIME type prefix for a file extension.
/// tower-http uses the `mime_guess` crate, which may serve either legacy or modern MIME types
/// depending on the version. We verify the *category* is correct (e.g., JS gets javascript).
fn expected_mime_contains(ext: &str) -> &'static str {
    match ext {
        "html" => "text/html",
        "css" => "text/css",
        "js" => "javascript",       // can be text/javascript or application/javascript
        "json" => "json",            // application/json
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "svg",              // image/svg+xml
        "ico" => "image/",           // image/x-icon or image/vnd.microsoft.icon
        "wasm" => "wasm",            // application/wasm
        "txt" => "text/plain",
        "xml" => "xml",              // text/xml or application/xml
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

/// Known extensions that tower-http/ServeDir will reliably detect.
const KNOWN_EXTENSIONS: &[&str] = &[
    "html", "css", "js", "json", "png", "jpg", "gif", "svg", "ico",
    "wasm", "txt", "xml", "webp",
];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// **Validates: Requirements 6.1, 6.4**
    ///
    /// For any known file extension served from the static directory,
    /// the response must have a Content-Type header matching the expected MIME category.
    #[test]
    fn static_files_have_correct_mime_type(
        ext_idx in 0..KNOWN_EXTENSIONS.len(),
        filename_base in "[a-z]{3,10}"
    ) {
        let ext = KNOWN_EXTENSIONS[ext_idx];
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tmp = tempfile::TempDir::new().unwrap();
            let static_dir = tmp.path().join("static");
            std::fs::create_dir_all(&static_dir).unwrap();

            // Create index.html (required for SPA fallback)
            std::fs::write(static_dir.join("index.html"), "<html></html>").unwrap();

            // Create test file with the given extension
            let filename = format!("{}.{}", filename_base, ext);
            let file_path = static_dir.join(&filename);
            std::fs::write(&file_path, "test content").unwrap();

            let state = stockenboard_lib::core_state::CoreState::new(tmp.path()).unwrap();
            let app = stockenboard_lib::api::build_router_with_static(
                Arc::new(state),
                &static_dir,
            );

            let uri = format!("/{}", filename);
            let response = app
                .oneshot(
                    Request::builder()
                        .uri(&uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                http::StatusCode::OK,
                "Expected 200 for file {}, got {}",
                filename,
                response.status()
            );

            let content_type = response
                .headers()
                .get("content-type")
                .map(|v| v.to_str().unwrap().to_string())
                .unwrap_or_default();

            let expected_substr = expected_mime_contains(ext);
            assert!(
                content_type.contains(expected_substr),
                "Expected Content-Type containing '{}' for .{} file, got '{}'",
                expected_substr,
                ext,
                content_type
            );
        });
    }

    /// **Validates: Requirements 6.3**
    ///
    /// For any file path matching the hashed asset pattern (assets/*-HASH.ext),
    /// the response must include `Cache-Control: public, max-age=31536000, immutable`.
    #[test]
    fn hashed_assets_get_immutable_cache_headers(
        hash in "[a-f0-9]{6,12}",
        name in "[a-z]{3,8}",
        ext_idx in 0..3usize  // js, css, wasm
    ) {
        let extensions = ["js", "css", "wasm"];
        let ext = extensions[ext_idx];
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tmp = tempfile::TempDir::new().unwrap();
            let static_dir = tmp.path().join("static");
            let assets_dir = static_dir.join("assets");
            std::fs::create_dir_all(&assets_dir).unwrap();

            // Create index.html (required for SPA fallback)
            std::fs::write(static_dir.join("index.html"), "<html></html>").unwrap();

            // Create a hashed asset file like "assets/chunk-a1b2c3.js"
            let filename = format!("{}-{}.{}", name, hash, ext);
            std::fs::write(assets_dir.join(&filename), "hashed asset content").unwrap();

            let state = stockenboard_lib::core_state::CoreState::new(tmp.path()).unwrap();
            let app = stockenboard_lib::api::build_router_with_static(
                Arc::new(state),
                &static_dir,
            );

            let uri = format!("/assets/{}", filename);
            let response = app
                .oneshot(
                    Request::builder()
                        .uri(&uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                http::StatusCode::OK,
                "Expected 200 for hashed asset {}, got {}",
                uri,
                response.status()
            );

            let cache_control = response
                .headers()
                .get("cache-control")
                .map(|v| v.to_str().unwrap().to_string())
                .unwrap_or_default();

            assert_eq!(
                cache_control,
                "public, max-age=31536000, immutable",
                "Hashed asset {} should have immutable cache header, got '{}'",
                uri,
                cache_control
            );
        });
    }

    /// **Validates: Requirements 6.3**
    ///
    /// index.html must get `Cache-Control: no-cache, no-store, must-revalidate`.
    /// Test both `/` and `/index.html` paths.
    #[test]
    fn index_html_gets_no_cache_headers(
        path_variant in prop_oneof![Just("/".to_string()), Just("/index.html".to_string())]
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tmp = tempfile::TempDir::new().unwrap();
            let static_dir = tmp.path().join("static");
            std::fs::create_dir_all(&static_dir).unwrap();

            // Create index.html
            std::fs::write(static_dir.join("index.html"), "<html><body>SPA</body></html>").unwrap();

            let state = stockenboard_lib::core_state::CoreState::new(tmp.path()).unwrap();
            let app = stockenboard_lib::api::build_router_with_static(
                Arc::new(state),
                &static_dir,
            );

            let response = app
                .oneshot(
                    Request::builder()
                        .uri(path_variant.as_str())
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                http::StatusCode::OK,
                "Expected 200 for {}, got {}",
                path_variant,
                response.status()
            );

            let cache_control = response
                .headers()
                .get("cache-control")
                .map(|v| v.to_str().unwrap().to_string())
                .unwrap_or_default();

            assert_eq!(
                cache_control,
                "no-cache, no-store, must-revalidate",
                "index.html at '{}' should have no-cache header, got '{}'",
                path_variant,
                cache_control
            );
        });
    }

    /// **Validates: Requirements 6.3**
    ///
    /// Non-hashed, non-index static files should NOT have incorrect cache headers.
    /// Specifically, they must not receive immutable or no-store directives.
    #[test]
    fn non_hashed_non_index_files_no_incorrect_cache(
        filename in "[a-z]{3,8}\\.(txt|png|jpg|css|js)"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tmp = tempfile::TempDir::new().unwrap();
            let static_dir = tmp.path().join("static");
            std::fs::create_dir_all(&static_dir).unwrap();

            // Create index.html (required for SPA fallback)
            std::fs::write(static_dir.join("index.html"), "<html></html>").unwrap();

            // Create the test file at root level (not under assets/)
            std::fs::write(static_dir.join(&filename), "regular file").unwrap();

            let state = stockenboard_lib::core_state::CoreState::new(tmp.path()).unwrap();
            let app = stockenboard_lib::api::build_router_with_static(
                Arc::new(state),
                &static_dir,
            );

            let uri = format!("/{}", filename);
            let response = app
                .oneshot(
                    Request::builder()
                        .uri(&uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                http::StatusCode::OK,
                "Expected 200 for file {}, got {}",
                filename,
                response.status()
            );

            let cache_control = response
                .headers()
                .get("cache-control")
                .map(|v| v.to_str().unwrap().to_string());

            // Non-hashed, non-index files should not get immutable or no-store headers
            if let Some(ref cc) = cache_control {
                assert!(
                    !cc.contains("immutable"),
                    "Non-hashed file '{}' should not have immutable cache, got '{}'",
                    filename,
                    cc
                );
                assert!(
                    !cc.contains("no-store"),
                    "Non-index file '{}' should not have no-store cache, got '{}'",
                    filename,
                    cc
                );
            }
            // cache_control being None is also acceptable (no explicit override)
        });
    }
}
