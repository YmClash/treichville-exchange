use std::sync::Arc;
use std::net::SocketAddr;
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tokio::signal;

mod domain;
mod application;
mod infrastructure;
mod presentation;
mod shared;

use crate::{
    shared::{
        config::Config,
        state::AppState,
    },
    application::{
        auth::{AuthService, JwtService, UserRepository},
        rates::{RateService, RateRepository, RateCache, RateAggregator},
        exchange::{
            TransactionService, TransactionRepository, 
            MatchingEngine, WalletService, PaymentProcessor,
        },
    },
    presentation::rest::{routes::create_router, handlers::health::HealthState},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "treichville_exchange=debug,tower_http=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Treichville Exchange Backend...");

    // Load configuration
    let config = Config::from_env()?;
    info!("Configuration loaded successfully");

    // Connect to PostgreSQL
    let db_pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .acquire_timeout(std::time::Duration::from_secs(config.database.acquire_timeout))
        .connect(&config.database.url)
        .await?;
    
    info!("Connected to PostgreSQL");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await?;
    
    info!("Database migrations completed");

    // Connect to Redis
    let redis_client = redis::Client::open(config.redis.url.clone())?;
    let redis_conn = redis::aio::ConnectionManager::new(redis_client).await?;
    let redis_arc = Arc::new(redis_conn);
    
    info!("Connected to Redis");

    // Initialize repositories
    let user_repository = Arc::new(UserRepository::new(db_pool.clone()));
    let rate_repository = Arc::new(RateRepository::new(db_pool.clone()));
    let transaction_repository = Arc::new(TransactionRepository::new(db_pool.clone()));
    
    // Initialize caches
    let rate_cache = Arc::new(RateCache::new(redis_arc.clone()));
    
    // Initialize services
    let jwt_service = Arc::new(JwtService::new(
        config.jwt.secret.clone(),
        config.jwt.access_token_expiry,
        config.jwt.refresh_token_expiry,
    ));
    
    let auth_service = Arc::new(AuthService::new(
        user_repository.clone(),
        jwt_service.clone(),
        redis_arc.clone(),
    ));
    
    let rate_aggregator = Arc::new(RateAggregator::new(
        rate_repository.clone(),
        rate_cache.clone(),
    ));
    
    let rate_service = Arc::new(RateService::new(
        rate_repository.clone(),
        rate_cache.clone(),
        rate_aggregator.clone(),
    ));
    
    let matching_engine = Arc::new(MatchingEngine::new(
        rate_repository.clone(),
        user_repository.clone(),
    ));
    
    let wallet_service = Arc::new(WalletService::new(
        db_pool.clone(),
        redis_arc.clone(),
    ));
    
    let payment_processor = Arc::new(PaymentProcessor::new(
        transaction_repository.clone(),
        wallet_service.clone(),
        config.clone(),
    ));
    
    let transaction_service = Arc::new(TransactionService::new(
        transaction_repository.clone(),
        matching_engine.clone(),
        wallet_service.clone(),
        payment_processor.clone(),
        rate_service.clone(),
    ));
    
    info!("All services initialized");

    // Create application state
    let app_state = Arc::new(AppState {
        config: config.clone(),
        db: db_pool.clone(),
        redis: redis_arc.clone(),
        auth_service: auth_service.clone(),
        rate_service: rate_service.clone(),
        transaction_service: transaction_service.clone(),
        wallet_service: wallet_service.clone(),
        jwt_service: jwt_service.clone(),
    });

    // Create health check state
    let health_state = Arc::new(HealthState {
        db: db_pool.clone(),
        redis: redis_arc.clone(),
        start_time: std::time::Instant::now(),
    });

    // Create the main router
    let app = create_router(app_state.clone())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(health_state);

    // Bind to address
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    info!("Server listening on {}", addr);

    // Create the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    // Start background tasks
    tokio::spawn(background_tasks(
        transaction_service.clone(),
        rate_aggregator.clone(),
    ));

    // Run the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received terminate signal");
        },
    }
}

async fn background_tasks(
    transaction_service: Arc<TransactionService>,
    rate_aggregator: Arc<RateAggregator>,
) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
    
    loop {
        interval.tick().await;
        
        // Process expired transactions
        match transaction_service.process_expired_transactions().await {
            Ok(count) if count > 0 => {
                info!("Processed {} expired transactions", count);
            }
            Err(e) => {
                error!("Error processing expired transactions: {}", e);
            }
            _ => {}
        }
        
        // Update rate aggregations
        match rate_aggregator.update_all_aggregations().await {
            Ok(_) => {
                info!("Rate aggregations updated");
            }
            Err(e) => {
                error!("Error updating rate aggregations: {}", e);
            }
        }
    }
}
