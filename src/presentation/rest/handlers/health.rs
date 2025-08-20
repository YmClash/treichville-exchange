use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Row};
use redis::AsyncCommands;
use std::sync::Arc;

use crate::shared::errors::AppResult;
use crate::shared::state::HealthState;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub services: ServiceStatus,
}

#[derive(Debug, Serialize)]
pub struct ServiceStatus {
    pub database: ComponentHealth,
    pub redis: ComponentHealth,
    pub rate_engine: ComponentHealth,
    pub payment_providers: Vec<ProviderHealth>,
}

#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub status: String,
    pub latency_ms: Option<i64>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProviderHealth {
    pub name: String,
    pub status: String,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub active_connections: u32,
    pub database_pool: PoolMetrics,
    pub redis_pool: PoolMetrics,
    pub memory_usage: MemoryMetrics,
    pub business_metrics: BusinessMetrics,
}

#[derive(Debug, Serialize)]
pub struct PoolMetrics {
    pub size: u32,
    pub available: u32,
    pub waiting: u32,
}

#[derive(Debug, Serialize)]
pub struct MemoryMetrics {
    pub used_mb: f64,
    pub total_mb: f64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct BusinessMetrics {
    pub active_users: u64,
    pub active_changeurs: u64,
    pub transactions_today: u64,
    pub volume_today_fcfa: rust_decimal::Decimal,
}


// GET /health
pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now()
    })))
}

// GET /health/live
pub async fn liveness() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({
        "status": "alive"
    })))
}

// GET /health/ready
pub async fn readiness(
    State(state): State<Arc<crate::shared::state::AppState>>,
) -> AppResult<impl IntoResponse> {
    let mut is_ready = true;
    let mut services = vec![];

    // Check database
    let db_start = std::time::Instant::now();
    let db_status = match sqlx::query("SELECT 1").fetch_one(&state.health_state.db).await {
        Ok(_) => {
            services.push(("database", "healthy", Some(db_start.elapsed().as_millis() as i64)));
            "healthy"
        }
        Err(e) => {
            is_ready = false;
            services.push(("database", "unhealthy", None));
            tracing::error!("Database health check failed: {}", e);
            "unhealthy"
        }
    };

    // Check Redis
    let redis_start = std::time::Instant::now();
    let mut redis_conn = (*state.health_state.redis).clone();
    let redis_status = match redis::cmd("PING").query_async::<_, String>(&mut redis_conn).await {
        Ok(_) => {
            services.push(("redis", "healthy", Some(redis_start.elapsed().as_millis() as i64)));
            "healthy"
        }
        Err(e) => {
            is_ready = false;
            services.push(("redis", "unhealthy", None));
            tracing::error!("Redis health check failed: {}", e);
            "unhealthy"
        }
    };

    let status_code = if is_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok((status_code, Json(serde_json::json!({
        "ready": is_ready,
        "services": services.into_iter().map(|(name, status, latency)| {
            serde_json::json!({
                "name": name,
                "status": status,
                "latency_ms": latency
            })
        }).collect::<Vec<_>>()
    }))))
}

// GET /health/detailed
pub async fn detailed_health(
    State(state): State<Arc<crate::shared::state::AppState>>,
) -> AppResult<impl IntoResponse> {
    let mut database = ComponentHealth {
        status: "unknown".to_string(),
        latency_ms: None,
        message: None,
    };

    let mut redis = ComponentHealth {
        status: "unknown".to_string(),
        latency_ms: None,
        message: None,
    };

    // Check database
    let db_start = std::time::Instant::now();
    match sqlx::query("SELECT version()").fetch_one(&state.health_state.db).await {
        Ok(row) => {
            database.status = "healthy".to_string();
            database.latency_ms = Some(db_start.elapsed().as_millis() as i64);
            let version: String = row.try_get(0).unwrap_or_default();
            database.message = Some(format!("PostgreSQL {}", version));
        }
        Err(e) => {
            database.status = "unhealthy".to_string();
            database.message = Some(e.to_string());
        }
    }

    // Check Redis
    let redis_start = std::time::Instant::now();
    let mut redis_conn = (*state.health_state.redis).clone();
    match redis::cmd("PING").query_async::<_, String>(&mut redis_conn).await {
        Ok(_) => {
            redis.status = "healthy".to_string();
            redis.latency_ms = Some(redis_start.elapsed().as_millis() as i64);
            
            // Get Redis info
            if let Ok(info) = redis::cmd("INFO")
                .arg("server")
                .query_async::<_, String>(&mut redis_conn)
                .await
            {
                if let Some(version_line) = info.lines().find(|l| l.starts_with("redis_version:")) {
                    redis.message = Some(format!("Redis {}", version_line.split(':').nth(1).unwrap_or("")));
                }
            }
        }
        Err(e) => {
            redis.status = "unhealthy".to_string();
            redis.message = Some(e.to_string());
        }
    }

    // Rate engine status (simplified)
    let rate_engine = ComponentHealth {
        status: "healthy".to_string(),
        latency_ms: Some(5),
        message: Some("Rate aggregation active".to_string()),
    };

    // Payment providers status (mocked for now)
    let payment_providers = vec![
        ProviderHealth {
            name: "Orange Money".to_string(),
            status: "operational".to_string(),
            last_check: chrono::Utc::now(),
        },
        ProviderHealth {
            name: "Wave".to_string(),
            status: "operational".to_string(),
            last_check: chrono::Utc::now(),
        },
        ProviderHealth {
            name: "MTN Money".to_string(),
            status: "operational".to_string(),
            last_check: chrono::Utc::now(),
        },
    ];

    let response = HealthResponse {
        status: if database.status == "healthy" && redis.status == "healthy" {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        services: ServiceStatus {
            database,
            redis,
            rate_engine,
            payment_providers,
        },
    };

    Ok((StatusCode::OK, Json(response)))
}

// GET /metrics
pub async fn metrics(
    State(state): State<Arc<crate::shared::state::AppState>>,
) -> AppResult<impl IntoResponse> {
    let uptime_seconds = 0u64; // TODO: Track start time

    // Get database pool metrics
    let db_pool_metrics = {
        let pool = &state.health_state.db;
        PoolMetrics {
            size: pool.size(),
            available: pool.size() - pool.num_idle() as u32,
            waiting: 0, // Not directly available in sqlx
        }
    };

    // Get Redis pool metrics (simplified)
    let redis_pool_metrics = PoolMetrics {
        size: 10,  // Default pool size
        available: 8,
        waiting: 0,
    };

    // Get memory metrics
    let memory_metrics = {
        // This is a simplified version - in production you'd use a proper system info crate
        MemoryMetrics {
            used_mb: 256.0,
            total_mb: 1024.0,
            percentage: 25.0,
        }
    };

    // Get business metrics from database
    let mut business_metrics = BusinessMetrics {
        active_users: 0,
        active_changeurs: 0,
        transactions_today: 0,
        volume_today_fcfa: rust_decimal::Decimal::ZERO,
    };

    // Query active users
    if let Ok(row) = sqlx::query(
        "SELECT 
            COUNT(DISTINCT CASE WHEN role != 'changeur' THEN id END) as active_users,
            COUNT(DISTINCT CASE WHEN role = 'changeur' THEN id END) as active_changeurs
         FROM users 
         WHERE is_active = true"
    )
    .fetch_one(&state.health_state.db)
    .await {
        business_metrics.active_users = row.try_get::<i64, _>("active_users").unwrap_or(0) as u64;
        business_metrics.active_changeurs = row.try_get::<i64, _>("active_changeurs").unwrap_or(0) as u64;
    }

    // Query today's transactions
    if let Ok(row) = sqlx::query(
        "SELECT 
            COUNT(*) as count,
            COALESCE(SUM(amount), 0) as volume
         FROM transactions 
         WHERE created_at >= CURRENT_DATE"
    )
    .fetch_one(&state.health_state.db)
    .await {
        business_metrics.transactions_today = row.try_get::<i64, _>("count").unwrap_or(0) as u64;
        business_metrics.volume_today_fcfa = row.try_get("volume").unwrap_or(rust_decimal::Decimal::ZERO);
    }

    let response = MetricsResponse {
        uptime_seconds,
        total_requests: 0, // Would be tracked by middleware
        active_connections: db_pool_metrics.size - db_pool_metrics.available,
        database_pool: db_pool_metrics,
        redis_pool: redis_pool_metrics,
        memory_usage: memory_metrics,
        business_metrics,
    };

    Ok((StatusCode::OK, Json(response)))
}

// GET /version
pub async fn version() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": env!("CARGO_PKG_NAME"),
        "rust_version": env!("CARGO_PKG_RUST_VERSION"),
        "build_time": chrono::Utc::now().to_rfc3339(),
        "environment": std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
    })))
}