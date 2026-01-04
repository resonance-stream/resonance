//! Security headers middleware for Resonance API
//!
//! Adds HTTP security headers to all responses to protect against common
//! web vulnerabilities like clickjacking, MIME sniffing, and XSS attacks.
//!
//! Headers added:
//! - X-Frame-Options: Prevents clickjacking by disabling iframe embedding
//! - X-Content-Type-Options: Prevents MIME type sniffing
//! - Referrer-Policy: Controls referrer information sent with requests
//! - Content-Security-Policy: Restricts resource loading sources
//! - Permissions-Policy: Controls browser feature access

use axum::{
    body::Body,
    http::{header::HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};

// Define custom header names not in the standard library
static X_FRAME_OPTIONS: HeaderName = HeaderName::from_static("x-frame-options");
static X_CONTENT_TYPE_OPTIONS: HeaderName = HeaderName::from_static("x-content-type-options");
static REFERRER_POLICY: HeaderName = HeaderName::from_static("referrer-policy");
static CONTENT_SECURITY_POLICY: HeaderName = HeaderName::from_static("content-security-policy");
static PERMISSIONS_POLICY: HeaderName = HeaderName::from_static("permissions-policy");

/// Security headers middleware
///
/// Adds essential security headers to all HTTP responses:
///
/// - **X-Frame-Options: DENY** - Prevents the page from being embedded in iframes,
///   protecting against clickjacking attacks.
///
/// - **X-Content-Type-Options: nosniff** - Prevents browsers from MIME-sniffing
///   a response away from the declared content-type, mitigating drive-by downloads.
///
/// - **Referrer-Policy: strict-origin-when-cross-origin** - Sends full referrer for
///   same-origin requests, only origin for cross-origin requests, and nothing for
///   downgrade requests (HTTPS to HTTP).
///
/// - **Content-Security-Policy** - Restricts sources for scripts, styles, images,
///   media, and connections. Configured for a music streaming SPA:
///   - `default-src 'self'` - Only allow resources from same origin by default
///   - `script-src 'self'` - Scripts only from same origin
///   - `style-src 'self' 'unsafe-inline'` - Styles from same origin + inline (for CSS-in-JS)
///   - `img-src 'self' data: blob:` - Images from same origin, data URIs, and blob URLs
///   - `media-src 'self' blob:` - Audio/video from same origin and blob URLs
///   - `connect-src 'self' ws: wss:` - XHR/fetch/WebSocket to same origin + WebSocket protocols
///   - `font-src 'self'` - Fonts from same origin only
///   - `frame-ancestors 'none'` - Disallow embedding in frames (like X-Frame-Options)
///
/// - **Permissions-Policy** - Disables potentially dangerous browser features:
///   - `camera=()` - Disable camera access
///   - `microphone=()` - Disable microphone access (audio streaming doesn't need recording)
///   - `geolocation=()` - Disable location tracking
///   - `payment=()` - Disable Payment Request API
///   - `usb=()` - Disable USB device access
///
/// # Example
///
/// ```ignore
/// use axum::{Router, middleware};
/// use resonance_api::middleware::security_headers;
///
/// let app = Router::new()
///     .route("/", get(handler))
///     .layer(middleware::from_fn(security_headers));
/// ```
pub async fn security_headers(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Prevent clickjacking - page cannot be embedded in iframes
    headers.insert(
        X_FRAME_OPTIONS.clone(),
        HeaderValue::from_static("DENY"),
    );

    // Prevent MIME type sniffing
    headers.insert(
        X_CONTENT_TYPE_OPTIONS.clone(),
        HeaderValue::from_static("nosniff"),
    );

    // Control referrer information
    headers.insert(
        REFERRER_POLICY.clone(),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Content Security Policy for a music streaming SPA
    // - Allow self for most resources
    // - Allow unsafe-inline for styles (needed for CSS-in-JS libraries)
    // - Allow blob: and data: for dynamically generated content (album art, audio buffers)
    // - Allow ws: and wss: for WebSocket connections (real-time sync)
    // - Disallow embedding in frames
    headers.insert(
        CONTENT_SECURITY_POLICY.clone(),
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self'; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data: blob:; \
             media-src 'self' blob:; \
             connect-src 'self' ws: wss:; \
             font-src 'self'; \
             frame-ancestors 'none'"
        ),
    );

    // Permissions Policy - disable unnecessary browser features
    // This is the modern replacement for Feature-Policy
    headers.insert(
        PERMISSIONS_POLICY.clone(),
        HeaderValue::from_static(
            "camera=(), microphone=(), geolocation=(), payment=(), usb=()"
        ),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "OK"
    }

    fn create_test_app() -> Router {
        Router::new()
            .route("/", get(test_handler))
            .layer(axum::middleware::from_fn(security_headers))
    }

    #[tokio::test]
    async fn test_x_frame_options_header() {
        let app = create_test_app();
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("x-frame-options").unwrap(),
            "DENY"
        );
    }

    #[tokio::test]
    async fn test_x_content_type_options_header() {
        let app = create_test_app();
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(
            response.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
    }

    #[tokio::test]
    async fn test_referrer_policy_header() {
        let app = create_test_app();
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(
            response.headers().get("referrer-policy").unwrap(),
            "strict-origin-when-cross-origin"
        );
    }

    #[tokio::test]
    async fn test_content_security_policy_header() {
        let app = create_test_app();
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let csp = response
            .headers()
            .get("content-security-policy")
            .unwrap()
            .to_str()
            .unwrap();

        // Verify key CSP directives are present
        assert!(csp.contains("default-src 'self'"));
        assert!(csp.contains("script-src 'self'"));
        assert!(csp.contains("style-src 'self' 'unsafe-inline'"));
        assert!(csp.contains("img-src 'self' data: blob:"));
        assert!(csp.contains("media-src 'self' blob:"));
        assert!(csp.contains("connect-src 'self' ws: wss:"));
        assert!(csp.contains("font-src 'self'"));
        assert!(csp.contains("frame-ancestors 'none'"));
    }

    #[tokio::test]
    async fn test_permissions_policy_header() {
        let app = create_test_app();
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let permissions = response
            .headers()
            .get("permissions-policy")
            .unwrap()
            .to_str()
            .unwrap();

        // Verify key restrictions are present
        assert!(permissions.contains("camera=()"));
        assert!(permissions.contains("microphone=()"));
        assert!(permissions.contains("geolocation=()"));
        assert!(permissions.contains("payment=()"));
        assert!(permissions.contains("usb=()"));
    }

    #[tokio::test]
    async fn test_all_security_headers_present() {
        let app = create_test_app();
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Verify all 5 security headers are present
        assert!(response.headers().contains_key("x-frame-options"));
        assert!(response.headers().contains_key("x-content-type-options"));
        assert!(response.headers().contains_key("referrer-policy"));
        assert!(response.headers().contains_key("content-security-policy"));
        assert!(response.headers().contains_key("permissions-policy"));
    }
}
