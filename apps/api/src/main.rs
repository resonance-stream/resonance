use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::{ConnectInfo, Extension},
    http::{header, header::HeaderMap, Method},
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod graphql;
mod middleware;
mod models;
mod repositories;
mod routes;
mod services;
mod websocket;

pub use error::{ApiError, ApiResult, ErrorResponse};

use graphql::{build_schema, build_schema_with_rate_limiting, GraphQLRateLimiter, ResonanceSchema};
use middleware::{extract_client_ip, AuthRateLimitState};
use models::user::RequestMetadata;
use repositories::{SessionRepository, TrackRepository, UserRepository};
use routes::{
    auth_router, auth_router_with_rate_limiting, health_router, streaming_router, AuthState,
    HealthState, StreamingState,
};
use services::auth::{AuthConfig, AuthService};

/// Build the CORS layer based on configuration.
///
/// In production mode:
/// - If `CORS_ORIGINS` is set, only those origins are allowed
/// - If `CORS_ORIGINS` is not set, CORS requests are rejected (no origins allowed)
///
/// In development mode:
/// - If `CORS_ORIGINS` is set, those origins are used
/// - If `CORS_ORIGINS` is not set, permissive CORS is used for convenience
fn build_cors_layer(config: &config::Config) -> CorsLayer {
    let is_production = config.is_production();

    match &config.cors_allowed_origins {
        Some(origins) if !origins.is_empty() => {
            // Parse configured origins
            let allowed_origins: Vec<_> = origins
                .iter()
                .filter_map(|origin| {
                    origin.parse().ok().or_else(|| {
                        tracing::warn!("Invalid CORS origin '{}', skipping", origin);
                        None
                    })
                })
                .collect();

            if allowed_origins.is_empty() {
                tracing::error!("No valid CORS origins configured, CORS requests will be rejected");
                CorsLayer::new()
            } else {
                tracing::info!(
                    "CORS configured with {} allowed origin(s): {:?}",
                    allowed_origins.len(),
                    origins
                );
                CorsLayer::new()
                    .allow_origin(allowed_origins)
                    .allow_methods([
                        Method::GET,
                        Method::POST,
                        Method::PUT,
                        Method::PATCH,
                        Method::DELETE,
                        Method::OPTIONS,
                    ])
                    .allow_headers([
                        header::AUTHORIZATION,
                        header::CONTENT_TYPE,
                        header::ACCEPT,
                        header::ORIGIN,
                    ])
                    .allow_credentials(true)
                    .max_age(std::time::Duration::from_secs(3600))
            }
        }
        _ if is_production => {
            // Production without configured origins: strict CORS (no origins allowed)
            tracing::warn!(
                "CORS_ORIGINS not configured in production mode. \
                 CORS requests will be rejected. Set CORS_ORIGINS to allow cross-origin requests."
            );
            CorsLayer::new()
        }
        _ => {
            // Development without configured origins: permissive for convenience
            tracing::warn!(
                "Using permissive CORS in development mode. \
                 Set CORS_ORIGINS for production-like behavior."
            );
            CorsLayer::permissive()
        }
    }
}

/// Extract bearer token from Authorization header (case-insensitive)
fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?;

    // Split on whitespace and validate scheme case-insensitively
    let mut parts = value.split_whitespace();
    let scheme = parts.next()?;
    let token = parts.next()?;

    // Reject malformed values like "Bearer <token> <extra>"
    if parts.next().is_some() {
        return None;
    }

    if scheme.eq_ignore_ascii_case("bearer") && !token.is_empty() {
        Some(token)
    } else {
        None
    }
}

/// Extract user agent from headers
fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string())
}

/// GraphQL handler that executes queries against the schema
///
/// This handler extracts the Bearer token from the Authorization header,
/// verifies it using AuthService, and injects the Claims into the GraphQL
/// context so that queries like `me` and mutations like `logout` can access
/// the authenticated user's information.
///
/// It also extracts request metadata (IP address, user-agent) and injects
/// it into the context for auth mutations to use in session audit trails.
async fn graphql_handler(
    Extension(schema): Extension<ResonanceSchema>,
    Extension(auth_service): Extension<AuthService>,
    Extension(session_repo): Extension<SessionRepository>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut request = req.into_inner();

    // Extract request metadata for audit trails
    let ip_address = Some(extract_client_ip(&headers, connect_info.as_ref()));
    let user_agent = extract_user_agent(&headers);
    let request_metadata = RequestMetadata::new(ip_address, user_agent);

    // Inject RequestMetadata into the GraphQL context
    request = request.data(request_metadata);

    // Try to extract and verify the Bearer token
    if let Some(token) = extract_bearer_token(&headers) {
        match auth_service.verify_access_token(token) {
            Ok(claims) => {
                // Verify the session is still active in the database
                match session_repo.is_active(claims.sid, claims.sub).await {
                    Ok(true) => {
                        // Session is active, inject claims into context
                        request = request.data(claims);
                        tracing::debug!("GraphQL request authenticated");
                    }
                    Ok(false) => {
                        tracing::debug!(
                            user_id = %claims.sub,
                            "GraphQL session no longer active"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "GraphQL session check failed");
                    }
                }
            }
            Err(e) => {
                // Log the error but don't fail the request - unauthenticated
                // requests are allowed (they'll fail on protected resolvers)
                tracing::debug!(error = %e, "GraphQL auth token verification failed");
            }
        }
    }

    schema.execute(request).await.into()
}

/// GraphQL Playground handler for development
async fn graphql_playground() -> impl axum::response::IntoResponse {
    axum::response::Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    ))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "resonance_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Load configuration
    let config = config::Config::from_env()?;

    tracing::info!("Starting Resonance API server on port {}", config.port);

    // Initialize database pool
    let database_url = &config.common.database.url;
    tracing::info!("Connecting to database...");

    let pool = PgPoolOptions::new()
        .max_connections(config.common.database.max_connections)
        .acquire_timeout(std::time::Duration::from_secs(
            config.common.database.connect_timeout_secs,
        ))
        .connect(database_url)
        .await?;

    tracing::info!("Database connection established");

    // Run migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Migrations completed successfully");

    // Create UserRepository for centralized user database operations
    let user_repo = UserRepository::new(pool.clone());
    tracing::info!("UserRepository initialized");

    // Create SessionRepository for session validation in auth middleware
    let session_repo = SessionRepository::new(pool.clone());
    tracing::info!("SessionRepository initialized");

    // Create TrackRepository for streaming endpoint
    let track_repo = TrackRepository::new(pool.clone());
    tracing::info!("TrackRepository initialized");

    // Create StreamingState for audio streaming
    let streaming_state = StreamingState::new(track_repo, config.common.music_library_path.clone());
    tracing::info!("StreamingState initialized");

    // Create AuthService
    let auth_config = AuthConfig::with_expiry_strings(
        config.jwt_secret.clone(),
        &config.jwt_access_expiry,
        &config.jwt_refresh_expiry,
    );
    let auth_service = AuthService::new(pool.clone(), auth_config);

    tracing::info!("AuthService initialized");

    // Create health check state
    let health_state = HealthState::new(config.clone());

    // Create auth router state
    let auth_state = AuthState::new(auth_service.clone());

    // Initialize Redis client for rate limiting
    let redis_url = config.redis().connection_url();
    let redis_client = match redis::Client::open(redis_url.as_str()) {
        Ok(client) => {
            // Test Redis connection
            match client.get_multiplexed_async_connection().await {
                Ok(mut conn) => {
                    let pong: Result<String, _> = redis::cmd("PING").query_async(&mut conn).await;
                    if pong.is_ok() {
                        tracing::info!("Redis connected for rate limiting");
                        Some(client)
                    } else {
                        tracing::warn!("Redis ping failed, rate limiting disabled");
                        None
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Redis connection failed, rate limiting disabled");
                    None
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Redis client creation failed, rate limiting disabled");
            None
        }
    };

    // Build the CORS layer from configuration
    let cors_layer = build_cors_layer(&config);

    // Build GraphQL schema and auth router - with or without rate limiting based on Redis availability
    let (schema, auth_routes) = match redis_client {
        Some(client) => {
            // Create rate limit state for REST endpoints
            let rate_limit_state = AuthRateLimitState::new(client.clone());
            tracing::info!(
                "REST auth rate limiting enabled: login={} req/{} sec, register={} req/{} sec",
                rate_limit_state.login_config.max_requests,
                rate_limit_state.login_config.window_secs,
                rate_limit_state.register_config.max_requests,
                rate_limit_state.register_config.window_secs,
            );

            // Create GraphQL rate limiter
            let graphql_rate_limiter = GraphQLRateLimiter::new(client);
            tracing::info!("GraphQL auth rate limiting enabled");

            // Build schema with rate limiting
            let schema = build_schema_with_rate_limiting(
                pool.clone(),
                auth_service.clone(),
                graphql_rate_limiter,
            );
            tracing::info!("GraphQL schema built with rate limiting");

            let auth_routes = auth_router_with_rate_limiting(auth_state, rate_limit_state);
            (schema, auth_routes)
        }
        None => {
            tracing::warn!(
                "Auth rate limiting DISABLED - configure Redis (REDIS_URL) to enable protection against brute-force attacks"
            );

            // Build schema without rate limiting
            let schema = build_schema(pool.clone(), auth_service.clone());
            tracing::info!("GraphQL schema built (rate limiting disabled)");

            let auth_routes = auth_router(auth_state);
            (schema, auth_routes)
        }
    };

    // Build the router
    let app = Router::new()
        .route("/", get(root))
        // GraphQL endpoints
        .route("/graphql", post(graphql_handler))
        .route("/graphql/playground", get(graphql_playground))
        // Nested health routes: /health, /health/live, /health/ready
        .nest("/health", health_router(health_state))
        // Auth REST routes: /auth/register, /auth/login, /auth/refresh, /auth/logout
        .nest("/auth", auth_routes)
        // Streaming routes: /stream/:track_id
        .nest("/stream", streaming_router(streaming_state))
        // Add services as extensions for middleware extractors
        .layer(Extension(schema))
        .layer(Extension(pool.clone()))
        .layer(Extension(user_repo))
        .layer(Extension(session_repo))
        .layer(Extension(auth_service))
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer);

    // Run the server with ConnectInfo to capture client addresses
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("Listening on {}", addr);
    tracing::info!(
        "GraphQL Playground available at http://{}:{}/graphql/playground",
        addr.ip(),
        addr.port()
    );

    // Use into_make_service_with_connect_info to enable ConnectInfo extractor
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn root() -> &'static str {
    "Welcome to Resonance - Self-hosted Music Streaming"
}
