//! Static file serving layer with SPA fallback and cache-control headers.
//!
//! - Serves built SPA assets from a configurable directory (default: `./static`)
//! - Falls back to `index.html` for paths not matching static files (SPA client-side routing)
//! - Sets `Cache-Control: public, max-age=31536000, immutable` for hashed assets
//! - Sets `Cache-Control: no-cache, no-store, must-revalidate` for `index.html`
//! - Serves correct MIME types based on file extension

use axum::{
    body::Body,
    http::{header, Request, Response},
    Router,
};
use std::path::Path;
use tower::ServiceBuilder;
use tower_http::services::{ServeDir, ServeFile};

/// Header value for hashed/immutable assets (e.g., `assets/index-a1b2c3.js`)
const CACHE_IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// Header value for index.html (must always revalidate)
const CACHE_NO_STORE: &str = "no-cache, no-store, must-revalidate";

/// Determines whether a request path corresponds to a hashed asset.
///
/// Hashed assets match the pattern `assets/*-*.EXT` where the filename contains
/// a dash followed by a hash before the extension (e.g., `assets/index-a1b2c3.js`).
pub(crate) fn is_hashed_asset(path: &str) -> bool {
    // Normalize path separators and strip leading slash
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_start_matches('/');

    // Must start with "assets/"
    if !trimmed.starts_with("assets/") {
        return false;
    }

    // Get the filename portion after "assets/"
    let filename = &trimmed["assets/".len()..];

    // Must have an extension (contains a dot)
    if let Some(dot_pos) = filename.rfind('.') {
        let stem = &filename[..dot_pos];
        // Stem must contain a dash (e.g., "index-a1b2c3")
        stem.contains('-')
    } else {
        false
    }
}

/// Determines whether a request path is for `index.html`.
pub(crate) fn is_index_html(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_start_matches('/');
    trimmed.is_empty() || trimmed == "index.html"
}

/// Sets cache-control headers on a response based on the request path.
///
/// - Hashed assets → immutable, long-lived cache
/// - index.html or root → no-cache
/// - Other files → no explicit override
fn set_cache_headers(req_path: &str, response: &mut Response<Body>) {
    let cache_value = if is_hashed_asset(req_path) {
        Some(CACHE_IMMUTABLE)
    } else if is_index_html(req_path) {
        Some(CACHE_NO_STORE)
    } else {
        None
    };

    if let Some(value) = cache_value {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static(value),
        );
    }
}

/// Build the static file serving layer as a tower service with cache header injection.
///
/// Returns a Router that can be merged as a fallback into the main application router.
/// The router:
/// 1. Serves static files from `static_dir` with correct MIME types (handled by `ServeDir`)
/// 2. Falls back to `index.html` for non-matching paths (SPA client-side routing)
/// 3. Applies appropriate `Cache-Control` headers based on file path patterns
pub fn static_file_layer(static_dir: &Path) -> Router {
    Router::new().fallback_service(static_file_service(static_dir))
}

/// Build the raw static file serving service (without wrapping in a Router).
///
/// This returns the service directly for use with `Router::fallback_service()`.
/// It:
/// 1. Serves static files from `static_dir` with correct MIME types (handled by `ServeDir`)
/// 2. Falls back to `index.html` for non-matching paths (SPA client-side routing)
/// 3. Applies appropriate `Cache-Control` headers based on file path patterns
pub fn static_file_service(static_dir: &Path) -> axum::routing::MethodRouter {
    let index_path = static_dir.join("index.html");

    // Use .fallback() instead of .not_found_service() so the SPA fallback returns HTTP 200
    let serve_dir = ServeDir::new(static_dir)
        .fallback(ServeFile::new(index_path));

    // Use axum middleware to inject cache headers based on request URI
    axum::routing::any_service(
        ServiceBuilder::new()
            .layer(axum::middleware::from_fn(cache_header_middleware))
            .service(serve_dir),
    )
}

/// Axum middleware that injects Cache-Control headers based on the request path.
async fn cache_header_middleware(
    req: Request<Body>,
    next: axum::middleware::Next,
) -> Response<Body> {
    let path = req.uri().path().to_owned();
    let mut response = next.run(req).await;

    // Only set cache headers on successful responses
    if response.status().is_success() {
        set_cache_headers(&path, &mut response);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_hashed_asset_matching() {
        assert!(is_hashed_asset("/assets/index-a1b2c3.js"));
        assert!(is_hashed_asset("/assets/style-deadbeef.css"));
        assert!(is_hashed_asset("/assets/vendor-abc123.js"));
        assert!(is_hashed_asset("assets/chunk-9f8e7d.js"));
        assert!(is_hashed_asset("/assets/logo-hash123.svg"));
    }

    #[test]
    fn test_is_hashed_asset_non_matching() {
        // No hash in filename
        assert!(!is_hashed_asset("/assets/index.js"));
        // Not under assets/
        assert!(!is_hashed_asset("/other/index-abc.js"));
        // No extension
        assert!(!is_hashed_asset("/assets/index-abc"));
        // Root file
        assert!(!is_hashed_asset("/favicon.ico"));
        // index.html
        assert!(!is_hashed_asset("/index.html"));
    }

    #[test]
    fn test_is_index_html() {
        assert!(is_index_html("/index.html"));
        assert!(is_index_html("index.html"));
        assert!(is_index_html("/"));
        assert!(is_index_html(""));
    }

    #[test]
    fn test_is_not_index_html() {
        assert!(!is_index_html("/assets/index-abc.js"));
        assert!(!is_index_html("/favicon.ico"));
        assert!(!is_index_html("/about"));
    }
}
