use std::sync::Arc;
use sqlx::{Pool, Postgres};

#[derive(Clone)]
pub struct AppState {
    pub auth_service: Arc<crate::application::auth::AuthService>,
    pub rate_service: Arc<crate::application::rates::RateService>,
    pub transaction_service: Arc<crate::application::exchange::TransactionService>,
    pub wallet_service: Arc<crate::application::exchange::WalletService>,
    pub payment_processor: Arc<crate::application::exchange::PaymentProcessor>,
    pub health_state: Arc<HealthState>,
}

#[derive(Clone)]
pub struct HealthState {
    pub db: Pool<Postgres>,
    pub redis: Arc<redis::aio::ConnectionManager>,
    pub start_time: std::time::Instant,
}