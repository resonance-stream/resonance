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
//! - Strict-Transport-Security: Enforces HTTPS connections (production only)

use axum::{
    body::Body,
    extract::State,
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
static STRICT_TRANSPORT_SECURITY: HeaderName = HeaderName::from_static("strict-transport-security");

/// Configuration for security headers middleware
#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    /// Whether to enable HSTS (Strict-Transport-Security).
    /// Should only be enabled in production with HTTPS.
    pub enable_hsts: bool,

    /// HSTS max-age in seconds. Default is 1 year (31536000 seconds).
    /// This is the recommended value for production deployments.
    pub hsts_max_age: u64,

    /// Whether to include subdomains in HSTS policy.
    /// When true, adds "includeSubDomains" directive.
    pub hsts_include_subdomains: bool,

    /// Whether to enable HSTS preloading.
    /// Only set this if you're ready to submit to the HSTS preload list.
    /// See: https://hstspreload.org/
    pub hsts_preload: bool,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            enable_hsts: false,
            hsts_max_age: 31_536_000, // 1 year in seconds
            hsts_include_subdomains: true,
            hsts_preload: false,
        }
    }
}

impl SecurityHeadersConfig {
    /// Create configuration for production deployment.
    ///
    /// Enables HSTS with secure defaults:
    /// - 1 year max-age
    /// - includeSubDomains enabled
    /// - preload disabled (must be explicitly enabled)
    pub fn production() -> Self {
        Self {
            enable_hsts: true,
            hsts_max_age: 31_536_000,
            hsts_include_subdomains: true,
            hsts_preload: false,
        }
    }

    /// Create configuration for development (no HSTS).
    pub fn development() -> Self {
        Self::default()
    }

    /// Builder method to enable HSTS preloading.
    ///
    /// WARNING: Only enable this if you are certain that:
    /// 1. Your entire domain (including all subdomains) supports HTTPS
    /// 2. You want to submit your domain to the HSTS preload list
    /// 3. You understand this is difficult to reverse once preloaded
    #[allow(dead_code)]
    pub fn with_preload(mut self) -> Self {
        self.hsts_preload = true;
        self
    }

    /// Builder method to customize HSTS max-age.
    #[allow(dead_code)]
    pub fn with_max_age(mut self, seconds: u64) -> Self {
        self.hsts_max_age = seconds;
        self
    }

    /// Build the HSTS header value based on configuration.
    fn build_hsts_value(&self) -> String {
        let mut value = format!("max-age={}", self.hsts_max_age);

        if self.hsts_include_subdomains {
            value.push_str("; includeSubDomains");
        }

        if self.hsts_preload {
            value.push_str("; preload");
        }

        value
    }
}

/// Security headers middleware (development mode - no HSTS)
///
/// This is a convenience wrapper that uses default configuration without HSTS.
/// For production deployments with HTTPS, use [`security_headers_with_config`]
/// with [`SecurityHeadersConfig::production()`] instead.
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
///   - `object-src 'none'` - Disallow plugins (Flash, Java applets, etc.)
///   - `base-uri 'self'` - Restrict base element URLs to same origin
///   - `form-action 'self'` - Restrict form submissions to same origin
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
#[allow(dead_code)]
pub async fn security_headers(request: Request<Body>, next: Next) -> Response {
    apply_security_headers(request, next, &SecurityHeadersConfig::default()).await
}

/// Security headers middleware with configuration
///
/// Adds essential security headers to all HTTP responses. Use this version
/// when you need to enable HSTS for production deployments.
///
/// # Headers Added
///
/// All headers from [`security_headers`] plus:
///
/// - **Strict-Transport-Security** (when enabled) - Enforces HTTPS connections:
///   - `max-age=31536000` - Remember HSTS for 1 year
///   - `includeSubDomains` - Apply to all subdomains
///   - `preload` (optional) - Enable HSTS preload list submission
///
/// # Example
///
/// ```ignore
/// use axum::{Router, middleware};
/// use resonance_api::middleware::{security_headers_with_config, SecurityHeadersConfig};
///
/// // For production with HSTS
/// let config = SecurityHeadersConfig::production();
/// let app = Router::new()
///     .route("/", get(handler))
///     .layer(middleware::from_fn_with_state(config, security_headers_with_config));
///
/// // For development (no HSTS)
/// let config = SecurityHeadersConfig::development();
/// let app = Router::new()
///     .route("/", get(handler))
///     .layer(middleware::from_fn_with_state(config, security_headers_with_config));
/// ```
pub async fn security_headers_with_config(
    State(config): State<SecurityHeadersConfig>,
    request: Request<Body>,
    next: Next,
) -> Response {
    apply_security_headers(request, next, &config).await
}

/// Check if the request is using HTTPS based on x-forwarded-proto header or scheme.
///
/// In production behind a reverse proxy (nginx, Caddy, etc.), the proxy typically
/// terminates TLS and forwards requests to the backend over HTTP. The proxy sets
/// the `x-forwarded-proto` header to indicate the original protocol.
///
/// This function checks:
/// 1. The `x-forwarded-proto` header (set by reverse proxies)
/// 2. Falls back to checking if the request scheme is HTTPS
fn is_https_request(request: &Request<Body>) -> bool {
    // Check x-forwarded-proto header first (common in reverse proxy setups)
    if let Some(proto) = request.headers().get("x-forwarded-proto") {
        if let Ok(proto_str) = proto.to_str() {
            return proto_str.eq_ignore_ascii_case("https");
        }
    }

    // Fall back to checking the request scheme directly
    // This handles direct HTTPS connections without a proxy
    request
        .uri()
        .scheme_str()
        .is_some_and(|s| s.eq_ignore_ascii_case("https"))
}

/// Internal function that applies security headers based on configuration.
async fn apply_security_headers(
    request: Request<Body>,
    next: Next,
    config: &SecurityHeadersConfig,
) -> Response {
    // Check if request is HTTPS before consuming request
    let is_https = is_https_request(&request);

    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Prevent clickjacking - page cannot be embedded in iframes
    headers.insert(X_FRAME_OPTIONS.clone(), HeaderValue::from_static("DENY"));

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
    // - Disallow plugins (object-src 'none')
    // - Restrict base element URLs to prevent base tag hijacking
    // - Restrict form submissions to same origin to prevent form hijacking
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
             object-src 'none'; \
             base-uri 'self'; \
             form-action 'self'; \
             frame-ancestors 'none'",
        ),
    );

    // Permissions Policy - disable unnecessary browser features
    // This is the modern replacement for Feature-Policy
    headers.insert(
        PERMISSIONS_POLICY.clone(),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=(), usb=()"),
    );

    // HSTS (Strict-Transport-Security) - only for HTTPS requests in production
    // This header tells browsers to only connect via HTTPS for the specified duration.
    // Per RFC 6797, HSTS headers MUST be ignored on HTTP responses, so we only
    // send it when the request came via HTTPS (detected via x-forwarded-proto or scheme).
    if config.enable_hsts && is_https {
        let hsts_value = config.build_hsts_value();
        if let Ok(value) = HeaderValue::from_str(&hsts_value) {
            headers.insert(STRICT_TRANSPORT_SECURITY.clone(), value);
        }
    }

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
        assert_eq!(response.headers().get("x-frame-options").unwrap(), "DENY");
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
        assert!(csp.contains("object-src 'none'"));
        assert!(csp.contains("base-uri 'self'"));
        assert!(csp.contains("form-action 'self'"));
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

    #[tokio::test]
    async fn test_no_hsts_by_default() {
        let app = create_test_app();
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // HSTS should NOT be present by default (development mode)
        assert!(!response.headers().contains_key("strict-transport-security"));
    }

    fn create_test_app_with_config(config: SecurityHeadersConfig) -> Router {
        Router::new()
            .route("/", get(test_handler))
            .layer(axum::middleware::from_fn_with_state(
                config,
                security_headers_with_config,
            ))
    }

    #[tokio::test]
    async fn test_hsts_enabled_for_https_requests() {
        let config = SecurityHeadersConfig::production();
        let app = create_test_app_with_config(config);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-forwarded-proto", "https")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // HSTS should be present for HTTPS requests in production mode
        let hsts = response
            .headers()
            .get("strict-transport-security")
            .expect("HSTS header should be present for HTTPS requests")
            .to_str()
            .unwrap();

        // Verify default production settings
        assert!(hsts.contains("max-age=31536000"));
        assert!(hsts.contains("includeSubDomains"));
        assert!(!hsts.contains("preload")); // Not enabled by default
    }

    #[tokio::test]
    async fn test_hsts_not_present_for_http_requests_in_production() {
        let config = SecurityHeadersConfig::production();
        let app = create_test_app_with_config(config);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-forwarded-proto", "http")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // HSTS should NOT be present for HTTP requests (per RFC 6797)
        assert!(
            !response.headers().contains_key("strict-transport-security"),
            "HSTS header should not be present for HTTP requests"
        );
    }

    #[tokio::test]
    async fn test_hsts_not_present_without_proto_header() {
        let config = SecurityHeadersConfig::production();
        let app = create_test_app_with_config(config);
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // HSTS should NOT be present when x-forwarded-proto is not set
        // (and request scheme is not HTTPS)
        assert!(
            !response.headers().contains_key("strict-transport-security"),
            "HSTS header should not be present without HTTPS indication"
        );
    }

    #[tokio::test]
    async fn test_hsts_not_present_in_development() {
        let config = SecurityHeadersConfig::development();
        let app = create_test_app_with_config(config);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-forwarded-proto", "https")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // HSTS should NOT be present in development mode even for HTTPS
        assert!(!response.headers().contains_key("strict-transport-security"));
    }

    #[tokio::test]
    async fn test_hsts_with_preload() {
        let config = SecurityHeadersConfig::production().with_preload();
        let app = create_test_app_with_config(config);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-forwarded-proto", "https")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let hsts = response
            .headers()
            .get("strict-transport-security")
            .expect("HSTS header should be present for HTTPS requests")
            .to_str()
            .unwrap();

        // Verify preload is included
        assert!(hsts.contains("max-age=31536000"));
        assert!(hsts.contains("includeSubDomains"));
        assert!(hsts.contains("preload"));
    }

    #[tokio::test]
    async fn test_hsts_custom_max_age() {
        let config = SecurityHeadersConfig::production().with_max_age(86400); // 1 day
        let app = create_test_app_with_config(config);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-forwarded-proto", "https")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let hsts = response
            .headers()
            .get("strict-transport-security")
            .expect("HSTS header should be present for HTTPS requests")
            .to_str()
            .unwrap();

        // Verify custom max-age
        assert!(hsts.contains("max-age=86400"));
        assert!(hsts.contains("includeSubDomains"));
    }

    #[tokio::test]
    async fn test_x_forwarded_proto_https_case_insensitive() {
        let config = SecurityHeadersConfig::production();
        let app = create_test_app_with_config(config);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-forwarded-proto", "HTTPS")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // HSTS should be present even with uppercase HTTPS
        assert!(
            response.headers().contains_key("strict-transport-security"),
            "HSTS should be present for HTTPS (case-insensitive)"
        );
    }

    #[test]
    fn test_config_default() {
        let config = SecurityHeadersConfig::default();
        assert!(!config.enable_hsts);
        assert_eq!(config.hsts_max_age, 31_536_000);
        assert!(config.hsts_include_subdomains);
        assert!(!config.hsts_preload);
    }

    #[test]
    fn test_config_production() {
        let config = SecurityHeadersConfig::production();
        assert!(config.enable_hsts);
        assert_eq!(config.hsts_max_age, 31_536_000);
        assert!(config.hsts_include_subdomains);
        assert!(!config.hsts_preload);
    }

    #[test]
    fn test_config_development() {
        let config = SecurityHeadersConfig::development();
        assert!(!config.enable_hsts);
    }

    #[test]
    fn test_build_hsts_value_basic() {
        let config = SecurityHeadersConfig {
            enable_hsts: true,
            hsts_max_age: 31536000,
            hsts_include_subdomains: true,
            hsts_preload: false,
        };
        assert_eq!(
            config.build_hsts_value(),
            "max-age=31536000; includeSubDomains"
        );
    }

    #[test]
    fn test_build_hsts_value_with_preload() {
        let config = SecurityHeadersConfig {
            enable_hsts: true,
            hsts_max_age: 31536000,
            hsts_include_subdomains: true,
            hsts_preload: true,
        };
        assert_eq!(
            config.build_hsts_value(),
            "max-age=31536000; includeSubDomains; preload"
        );
    }

    #[test]
    fn test_build_hsts_value_no_subdomains() {
        let config = SecurityHeadersConfig {
            enable_hsts: true,
            hsts_max_age: 86400,
            hsts_include_subdomains: false,
            hsts_preload: false,
        };
        assert_eq!(config.build_hsts_value(), "max-age=86400");
    }
}
