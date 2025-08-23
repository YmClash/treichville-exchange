use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Instant;
use tower_http::trace::MakeSpan;
use tracing::{info, warn, error, Span};
use uuid::Uuid;

// Request ID middleware
pub async fn request_id_middleware(
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let request_id = Uuid::new_v4().to_string();
    if let Ok(header_value) = request_id.parse() {
        req.headers_mut().insert(
            "x-request-id",
            header_value,
        );
    }
    
    let response = next.run(req).await;
    Ok(response)
}

// Logging middleware
pub async fn logging_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();
    
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    
    info!(
        request_id = request_id,
        method = %method,
        uri = %uri,
        "Request started"
    );
    
    let response = next.run(req).await;
    let latency = start.elapsed();
    
    info!(
        request_id = request_id,
        method = %method,
        uri = %uri,
        status = response.status().as_u16(),
        latency_ms = latency.as_millis(),
        "Request completed"
    );
    
    Ok(response)
}

// Rate limiting middleware
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};

#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<String, Vec<SystemTime>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window,
        }
    }
    
    pub async fn check_rate_limit(&self, key: &str) -> bool {
        let mut requests = self.requests.write().await;
        let now = SystemTime::now();
        let window_start = now - self.window;
        
        let entry = requests.entry(key.to_string()).or_insert_with(Vec::new);
        
        // Remove old requests outside the window
        entry.retain(|&time| time > window_start);
        
        if entry.len() < self.max_requests {
            entry.push(now);
            true
        } else {
            false
        }
    }
}

pub async fn rate_limit_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract client identifier (IP or user ID)
    let client_id = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            req.headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
        })
        .unwrap_or("unknown")
        .to_string();
    
    // Check rate limit (would need to be injected as state)
    // For now, we'll skip the actual rate limiting implementation
    
    let response = next.run(req).await;
    Ok(response)
}

// Security headers middleware
pub async fn security_headers_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    
    // These header values are static strings and should always parse successfully
    // But we handle errors gracefully to avoid panics
    if let Ok(value) = "nosniff".parse() {
        headers.insert("X-Content-Type-Options", value);
    }
    if let Ok(value) = "DENY".parse() {
        headers.insert("X-Frame-Options", value);
    }
    if let Ok(value) = "1; mode=block".parse() {
        headers.insert("X-XSS-Protection", value);
    }
    if let Ok(value) = "max-age=31536000; includeSubDomains".parse() {
        headers.insert("Strict-Transport-Security", value);
    }
    if let Ok(value) = "default-src 'self'".parse() {
        headers.insert("Content-Security-Policy", value);
    }
    
    Ok(response)
}

// Error handling middleware
pub async fn error_handling_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let response = next.run(req).await;
    
    // Log errors
    if response.status().is_server_error() {
        error!("Server error: {}", response.status());
    } else if response.status().is_client_error() {
        warn!("Client error: {}", response.status());
    }
    
    Ok(response)
}

// Metrics middleware
use prometheus::{IntCounter, Histogram, HistogramOpts};
use lazy_static::lazy_static;

lazy_static! {
    static ref REQUEST_COUNTER: IntCounter = IntCounter::new(
        "http_requests_total",
        "Total number of HTTP requests"
    ).unwrap_or_else(|e| {
        error!("Failed to create REQUEST_COUNTER metric: {}", e);
        // Create a dummy counter that does nothing
        IntCounter::new("dummy", "dummy").expect("Dummy counter should always work")
    });
    
    static ref REQUEST_DURATION: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds"
        )
    ).unwrap_or_else(|e| {
        error!("Failed to create REQUEST_DURATION metric: {}", e);
        // Create a dummy histogram that does nothing
        Histogram::with_opts(
            HistogramOpts::new("dummy", "dummy")
        ).expect("Dummy histogram should always work")
    });
}

pub async fn metrics_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let start = Instant::now();
    REQUEST_COUNTER.inc();
    
    let response = next.run(req).await;
    
    let duration = start.elapsed().as_secs_f64();
    REQUEST_DURATION.observe(duration);
    
    Ok(response)
}

// CORS configuration helper
pub fn cors_layer() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{Any, CorsLayer};
    
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any)
        .max_age(Duration::from_secs(3600))
}

// Request body size limit
pub fn body_limit_layer() -> tower_http::limit::RequestBodyLimitLayer {
    tower_http::limit::RequestBodyLimitLayer::new(10 * 1024 * 1024) // 10MB
}

// Compression layer
pub fn compression_layer() -> tower_http::compression::CompressionLayer {
    tower_http::compression::CompressionLayer::new()
}

// Timeout layer
pub fn timeout_layer() -> tower::timeout::TimeoutLayer {
    tower::timeout::TimeoutLayer::new(Duration::from_secs(30))
}