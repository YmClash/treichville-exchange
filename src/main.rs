use std::sync::Arc;
use std::net::SocketAddr;
use std::str::FromStr;
use sqlx::postgres::PgPoolOptions;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tokio::signal;

mod config;
mod domain;
mod application;
mod infrastructure;
mod presentation;
mod shared;

use crate::{
    config::settings::Settings,
    shared::{
        config::Config,
        config_adapter::settings_to_config,
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
    presentation::rest::routes::create_router,
    shared::state::HealthState,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // console_subscriber::init();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "treichville_exchange=debug,tower_http=debug,axum=debug".into()),
        )
        .with(console_subscriber::spawn())
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Treichville Exchange Backend...");
    
    // Initialize validators
    shared::utils::init_validators();
    info!("Validators initialized");

    // Load configuration
    let settings = Settings::from_env()
        .map_err(|e| {
            error!("Failed to load configuration: {}", e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;
    let config = settings_to_config(&settings);
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

    // Connect to Redis with connection pool
    let redis_pool = Arc::new(
        infrastructure::redis_pool::RedisPool::new(config.redis.url.clone())
            .await
            .map_err(|e| {
                error!("Failed to create Redis pool: {}", e);
                Box::new(e) as Box<dyn std::error::Error>
            })?
    );
    
    // Get initial connection for services that need ConnectionManager
    let redis_conn = redis_pool.get_connection().await?;
    let redis_arc = Arc::new(redis_conn);
    
    info!("Connected to Redis with connection pool");

    // Initialize repositories
    let user_repository = Arc::new(UserRepository::new(db_pool.clone()));
    let rate_repository = Arc::new(RateRepository::new(db_pool.clone()));
    let transaction_repository = Arc::new(TransactionRepository::new(db_pool.clone()));
    
    // Initialize caches
    let rate_cache = Arc::new(RateCache::new((*redis_arc).clone()));
    
    // Initialize services
    let jwt_service = Arc::new(JwtService::new(
        config.jwt.clone(),
    ));
    
    let auth_service = Arc::new(AuthService::new(
        db_pool.clone(),
        (*redis_arc).clone(),
        config.clone(),
    ));
    
    let rate_aggregator = Arc::new(RateAggregator {});
    
    let default_fee = rust_decimal::Decimal::from_str("0.01")
        .unwrap_or_else(|e| {
            error!("Failed to parse default fee, using 0.01: {}", e);
            rust_decimal::Decimal::from(1) / rust_decimal::Decimal::from(100)
        });
    
    let rate_service = Arc::new(RateService::new(
        rate_repository.clone(),
        rate_cache.clone(),
        default_fee, // 1% fee
    ));
    
    let matching_engine = Arc::new(MatchingEngine::new());
    
    let wallet_service = Arc::new(WalletService::new(
        db_pool.clone(),
    ));
    
    let payment_processor = Arc::new(PaymentProcessor::new());
    
    let transaction_service = Arc::new(TransactionService::new(
        transaction_repository.clone(),
        user_repository.clone(),
        rate_service.clone(),
        wallet_service.clone(),
        payment_processor.clone(),
        config.clone(),
    ));
    
    info!("All services initialized");

    // Create health check state
    let health_state = Arc::new(HealthState {
        db: db_pool.clone(),
        redis: redis_arc.clone(),
        start_time: std::time::Instant::now(),
    });

    // Create application state
    let app_state = Arc::new(AppState {
        auth_service: auth_service.clone(),
        rate_service: rate_service.clone(),
        transaction_service: transaction_service.clone(),
        wallet_service: wallet_service.clone(),
        payment_processor: payment_processor.clone(),
        jwt_service: jwt_service.clone(),
        health_state: health_state.clone(),
    });

    // Create the main router
    let app = create_router(app_state.clone())
        .layer(tower_http::trace::TraceLayer::new_for_http());

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
        if let Err(e) = signal::ctrl_c().await {
            error!("Failed to install Ctrl+C handler: {}", e);
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => { sig.recv().await; },
            Err(e) => {
                error!("Failed to install signal handler: {}", e);
            }
        }
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
        
        // Update rate aggregations - TODO: Implement aggregation update
        // match rate_aggregator.update_all_aggregations().await {
        //     Ok(_) => {
        //         info!("Rate aggregations updated");
        //     }
        //     Err(e) => {
        //         error!("Error updating rate aggregations: {}", e);
        //     }
        // }
    }
}
