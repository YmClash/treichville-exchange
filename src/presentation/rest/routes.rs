use axum::{
    routing::{get, post, put, delete},
    Router,
    middleware,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
    limit::RequestBodyLimitLayer,
};

use crate::{
    application::auth::middleware::{AuthMiddleware, require_auth},
    presentation::rest::handlers::{auth, rates, exchange, wallet, health},
    shared::{state::AppState, sql_validator::SqlValidator},
};

pub fn create_router(state: Arc<AppState>) -> Router {
    // Public routes (no auth required)
    let public_routes = Router::new()
        // Health checks
        .route("/health", get(health::health_check))
        .route("/health/live", get(health::liveness))
        .route("/health/ready", get(health::readiness))
        .route("/health/detailed", get(health::detailed_health))
        .route("/metrics", get(health::metrics))
        .route("/metrics/prometheus", get(health::prometheus_metrics))
        .route("/version", get(health::version))
        
        // Auth endpoints
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/refresh", post(auth::refresh_token))
        .route("/api/v1/auth/verify-email/:token", get(auth::verify_email))
        .route("/api/v1/auth/forgot-password", post(auth::forgot_password))
        .route("/api/v1/auth/reset-password", post(auth::reset_password))
        
        // Public rate endpoints
        .route("/api/v1/rates/public", get(rates::get_public_rates))
        .route("/api/v1/rates/best", get(rates::get_best_rate))
        .route("/api/v1/rates/quote", get(rates::get_quote))
        .route("/api/v1/rates/market-depth", get(rates::get_market_depth))
        .route("/api/v1/rates/aggregated", get(rates::get_aggregated_rates))
        
        // Payment webhooks (verified by signature, not JWT)
        .route("/api/v1/exchange/webhook/:provider", post(exchange::payment_webhook));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        // Auth endpoints
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/auth/me", get(auth::get_current_user))
        .route("/api/v1/auth/update-profile", put(auth::update_profile))
        .route("/api/v1/auth/change-password", post(auth::change_password))
        .route("/api/v1/auth/enable-2fa", post(auth::enable_2fa))
        .route("/api/v1/auth/verify-2fa", post(auth::verify_2fa))
        .route("/api/v1/auth/disable-2fa", post(auth::disable_2fa))
        
        // Rate endpoints (changeur only)
        .route("/api/v1/rates", get(rates::get_rates))
        .route("/api/v1/rates", post(rates::create_rate))
        .route("/api/v1/rates/changeur", get(rates::get_changeur_rates))
        .route("/api/v1/rates/bulk", post(rates::bulk_update_rates))
        .route("/api/v1/rates/:id", put(rates::update_rate))
        .route("/api/v1/rates/:id", delete(rates::delete_rate))
        
        // Exchange endpoints
        .route("/api/v1/exchange/initiate", post(exchange::initiate_transaction))
        .route("/api/v1/exchange/confirm", post(exchange::confirm_payment))
        .route("/api/v1/exchange/:id/complete", post(exchange::complete_transaction))
        .route("/api/v1/exchange/:id/cancel", post(exchange::cancel_transaction))
        .route("/api/v1/exchange/:id", get(exchange::get_transaction))
        .route("/api/v1/exchange", get(exchange::get_user_transactions))
        .route("/api/v1/exchange/statistics", get(exchange::get_transaction_statistics))
        
        // Wallet endpoints
        .route("/api/v1/wallet/balance", get(wallet::get_balance))
        .route("/api/v1/wallet/balance/:currency", get(wallet::get_balance_by_currency))
        .route("/api/v1/wallet/deposit", post(wallet::deposit))
        .route("/api/v1/wallet/withdraw", post(wallet::withdraw))
        .route("/api/v1/wallet/transfer", post(wallet::transfer))
        .route("/api/v1/wallet/reserve", post(wallet::reserve_funds))
        .route("/api/v1/wallet/release", post(wallet::release_funds))
        .route("/api/v1/wallet/transactions", get(wallet::get_wallet_transactions))
        .route("/api/v1/wallet/statistics", get(wallet::get_wallet_statistics))
        .route("/api/v1/wallet/limits", get(wallet::get_wallet_limits))
        .route("/api/v1/wallet/pending-operations", get(wallet::get_pending_operations))
        .route("/api/v1/wallet/validate-operation", post(wallet::validate_operation))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state.clone());

    // Admin routes
    let admin_routes = Router::new()
        .route("/api/v1/admin/users", get(auth::admin_get_users))
        .route("/api/v1/admin/users/:id/activate", post(auth::admin_activate_user))
        .route("/api/v1/admin/users/:id/deactivate", post(auth::admin_deactivate_user))
        .route("/api/v1/admin/changeurs/verify", post(auth::admin_verify_changeur))
        .route("/api/v1/admin/exchange/expired/process", post(exchange::process_expired_transactions))
        .route("/api/v1/admin/wallet/reconcile", post(wallet::reconcile_wallet))
        .layer(middleware::from_fn_with_state(state.clone(), require_admin));

    // Combine all routes with middleware layers
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(admin_routes)
        // Add middlewares layer by layer for compatibility
        .layer(middleware::from_fn(add_security_headers))
        .layer(middleware::from_fn_with_state(state.clone(), check_sql_injection))
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
        .layer(super::middleware::cors_layer())
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

// Middleware to authenticate user
async fn auth_middleware(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    use crate::application::auth::middleware::CurrentUser;
    use crate::domain::user::UserRole;
    
    // Extract authorization header
    let auth_header = req.headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
    
    // Verify token - use access token, not refresh token
    let claims = state.jwt_service.verify_access_token(auth_header)
        .map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
    
    // Create CurrentUser from claims
    let current_user = CurrentUser {
        id: claims.user_id().map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?,
        phone: claims.phone.clone(),
        role: claims.role.clone(),
    };
    
    // Add user to request extensions
    req.extensions_mut().insert(current_user);
    
    Ok(next.run(req).await)
}

// Middleware to require admin role
async fn require_admin(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    use axum::RequestExt;
    use crate::application::auth::middleware::CurrentUser;
    
    // Extract the current user from extensions
    let user = req.extensions().get::<CurrentUser>().cloned();
    
    match user {
        Some(user) if user.is_admin() => Ok(next.run(req).await),
        Some(_) => Err(axum::http::StatusCode::FORBIDDEN),
        None => Err(axum::http::StatusCode::UNAUTHORIZED),
    }
}

// Middleware to add security headers
async fn add_security_headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    
    // Add security headers
    if let Ok(value) = "nosniff".parse() {
        headers.insert("X-Content-Type-Options", value);
    }
    if let Ok(value) = "DENY".parse() {
        headers.insert("X-Frame-Options", value);
    }
    if let Ok(value) = "1; mode=block".parse() {
        headers.insert("X-XSS-Protection", value);
    }
    if let Ok(value) = "strict-origin-when-cross-origin".parse() {
        headers.insert("Referrer-Policy", value);
    }
    if let Ok(value) = "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'".parse() {
        headers.insert("Content-Security-Policy", value);
    }
    
    response
}

// Middleware to check for SQL injection attempts
async fn check_sql_injection(
    axum::extract::State(_state): axum::extract::State<Arc<AppState>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    // Extract client IP
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .unwrap_or("unknown")
        .trim()
        .to_string();
    
    let uri = request.uri().clone();
    let path = uri.path().to_string();
    
    // Check query parameters for SQL injection
    if let Some(query_string) = uri.query() {
        // Use query string directly (already decoded by axum)
        let decoded = query_string;
        
        if SqlValidator::detect_sql_injection_patterns(&decoded) {
            tracing::error!(
                "SQL injection attempt detected from IP: {} on path: {} with query: {}",
                client_ip, path, query_string
            );
            
            return Err(axum::http::StatusCode::BAD_REQUEST);
        }
    }
    
    // Check sensitive headers for SQL injection
    let headers = request.headers();
    for (name, value) in headers {
        if matches!(name.as_str(), "x-api-key" | "x-custom-header") {
            if let Ok(value_str) = value.to_str() {
                if SqlValidator::detect_sql_injection_patterns(value_str) {
                    tracing::warn!(
                        "SQL injection attempt in header '{}' from IP: {}",
                        name, client_ip
                    );
                    return Err(axum::http::StatusCode::BAD_REQUEST);
                }
            }
        }
    }
    
    Ok(next.run(request).await)
}

// WebSocket routes (to be implemented)
pub fn create_websocket_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ws/rates", get(websocket_handler))
        .route("/ws/transactions", get(websocket_handler))
        .with_state(state)
}

// Placeholder for WebSocket handler
async fn websocket_handler() -> impl axum::response::IntoResponse {
    "WebSocket endpoint - To be implemented"
}