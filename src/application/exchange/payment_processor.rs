use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::{
    domain::transaction::{PaymentMethod, Transaction},
    shared::errors::AppError,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PaymentProvider {
    OrangeMoney,
    Wave,
    MtnMoney,
    MoovMoney,
    BankTransfer,
    Cash,
}

impl From<PaymentMethod> for PaymentProvider {
    fn from(method: PaymentMethod) -> Self {
        match method {
            PaymentMethod::OrangeMoney => PaymentProvider::OrangeMoney,
            PaymentMethod::Wave => PaymentProvider::Wave,
            PaymentMethod::MtnMoney => PaymentProvider::MtnMoney,
            PaymentMethod::MoovMoney => PaymentProvider::MoovMoney,
            PaymentMethod::BankTransfer => PaymentProvider::BankTransfer,
            PaymentMethod::Cash => PaymentProvider::Cash,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub transaction_id: Uuid,
    pub provider: PaymentProvider,
    pub amount: Decimal,
    pub currency: String,
    pub sender_phone: String,
    pub recipient_phone: Option<String>,
    pub reference: String,
    pub description: String,
    pub callback_url: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResponse {
    pub transaction_id: Uuid,
    pub provider_reference: String,
    pub status: PaymentStatus,
    pub amount: Decimal,
    pub currency: String,
    pub fee: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
    pub receipt_url: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentStatus {
    Pending,
    Processing,
    Success,
    Failed,
    Cancelled,
    Expired,
    Refunded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentWebhook {
    pub provider: PaymentProvider,
    pub provider_reference: String,
    pub status: PaymentStatus,
    pub amount: Decimal,
    pub timestamp: DateTime<Utc>,
    pub signature: String,
    pub raw_data: serde_json::Value,
}

#[async_trait]
pub trait PaymentProviderTrait: Send + Sync {
    async fn initiate_payment(&self, request: &PaymentRequest) -> Result<PaymentResponse, AppError>;
    async fn check_status(&self, provider_reference: &str) -> Result<PaymentStatus, AppError>;
    async fn cancel_payment(&self, provider_reference: &str) -> Result<(), AppError>;
    async fn refund_payment(&self, provider_reference: &str, amount: Option<Decimal>) -> Result<PaymentResponse, AppError>;
    async fn verify_webhook(&self, webhook: &PaymentWebhook) -> Result<bool, AppError>;
}

pub struct PaymentProcessor {
    providers: HashMap<PaymentProvider, Box<dyn PaymentProviderTrait>>,
    webhook_secrets: HashMap<PaymentProvider, String>,
    retry_config: RetryConfig,
}

#[derive(Debug, Clone)]
struct RetryConfig {
    max_attempts: usize,
    initial_delay_ms: u64,
    max_delay_ms: u64,
    exponential_base: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            exponential_base: 2.0,
        }
    }
}

impl PaymentProcessor {
    pub fn new() -> Self {
        let mut providers: HashMap<PaymentProvider, Box<dyn PaymentProviderTrait>> = HashMap::new();
        
        // Initialize providers
        providers.insert(PaymentProvider::OrangeMoney, Box::new(OrangeMoneyProvider::new()));
        providers.insert(PaymentProvider::Wave, Box::new(WaveProvider::new()));
        providers.insert(PaymentProvider::MtnMoney, Box::new(MtnMoneyProvider::new()));
        providers.insert(PaymentProvider::Cash, Box::new(CashProvider::new()));

        Self {
            providers,
            webhook_secrets: HashMap::new(),
            retry_config: RetryConfig::default(),
        }
    }

    pub async fn process_payment(&self, request: PaymentRequest) -> Result<PaymentResponse, AppError> {
        let provider = self.providers
            .get(&request.provider)
            .ok_or_else(|| AppError::bad_request("Unsupported payment provider"))?;

        info!(
            transaction_id = %request.transaction_id,
            provider = ?request.provider,
            amount = %request.amount,
            "Processing payment"
        );

        // Attempt payment with retry logic
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < self.retry_config.max_attempts {
            match provider.initiate_payment(&request).await {
                Ok(response) => {
                    info!(
                        transaction_id = %request.transaction_id,
                        status = ?response.status,
                        "Payment processed successfully"
                    );
                    return Ok(response);
                }
                Err(e) => {
                    attempts += 1;
                    last_error = Some(e);
                    
                    if attempts < self.retry_config.max_attempts {
                        let delay = self.calculate_retry_delay(attempts);
                        warn!(
                            transaction_id = %request.transaction_id,
                            attempt = attempts,
                            delay_ms = delay,
                            "Payment failed, retrying..."
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        error!(
            transaction_id = %request.transaction_id,
            attempts = attempts,
            "Payment processing failed after all retries"
        );

        Err(last_error.unwrap_or_else(|| AppError::payment("Payment processing failed")))
    }

    pub async fn check_payment_status(
        &self,
        provider: PaymentProvider,
        provider_reference: &str,
    ) -> Result<PaymentStatus, AppError> {
        let provider_impl = self.providers
            .get(&provider)
            .ok_or_else(|| AppError::bad_request("Unsupported payment provider"))?;

        provider_impl.check_status(provider_reference).await
    }

    pub async fn handle_webhook(&self, webhook: PaymentWebhook) -> Result<(), AppError> {
        // Verify webhook signature
        let provider = self.providers
            .get(&webhook.provider)
            .ok_or_else(|| AppError::bad_request("Unknown provider"))?;

        if !provider.verify_webhook(&webhook).await? {
            warn!(
                provider = ?webhook.provider,
                reference = webhook.provider_reference,
                "Invalid webhook signature"
            );
            return Err(AppError::authentication("Invalid webhook signature"));
        }

        info!(
            provider = ?webhook.provider,
            reference = webhook.provider_reference,
            status = ?webhook.status,
            "Webhook processed"
        );

        // TODO: Update transaction status in database

        Ok(())
    }

    pub async fn refund_payment(
        &self,
        provider: PaymentProvider,
        provider_reference: &str,
        amount: Option<Decimal>,
    ) -> Result<PaymentResponse, AppError> {
        let provider_impl = self.providers
            .get(&provider)
            .ok_or_else(|| AppError::bad_request("Unsupported payment provider"))?;

        info!(
            provider = ?provider,
            reference = provider_reference,
            amount = ?amount,
            "Processing refund"
        );

        provider_impl.refund_payment(provider_reference, amount).await
    }

    fn calculate_retry_delay(&self, attempt: usize) -> u64 {
        let delay = self.retry_config.initial_delay_ms as f64 
            * self.retry_config.exponential_base.powi(attempt as i32 - 1);
        
        delay.min(self.retry_config.max_delay_ms as f64) as u64
    }
}

// Provider implementations (stubs for now)

struct OrangeMoneyProvider {
    api_url: String,
    client_id: String,
    client_secret: String,
}

impl OrangeMoneyProvider {
    fn new() -> Self {
        Self {
            api_url: std::env::var("ORANGE_MONEY_API_URL")
                .unwrap_or_else(|_| "https://api.orange.com/orange-money-webpay/dev/v1".to_string()),
            client_id: std::env::var("ORANGE_MONEY_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("ORANGE_MONEY_CLIENT_SECRET").unwrap_or_default(),
        }
    }
}

#[async_trait]
impl PaymentProviderTrait for OrangeMoneyProvider {
    async fn initiate_payment(&self, request: &PaymentRequest) -> Result<PaymentResponse, AppError> {
        // TODO: Implement actual Orange Money API call
        info!("Orange Money payment initiated for amount {}", request.amount);
        
        Ok(PaymentResponse {
            transaction_id: request.transaction_id,
            provider_reference: format!("OM-{}", Uuid::new_v4()),
            status: PaymentStatus::Pending,
            amount: request.amount,
            currency: request.currency.clone(),
            fee: Some(request.amount * Decimal::new(15, 3)), // 1.5% fee
            timestamp: Utc::now(),
            receipt_url: None,
            error_message: None,
        })
    }

    async fn check_status(&self, provider_reference: &str) -> Result<PaymentStatus, AppError> {
        // TODO: Implement actual status check
        Ok(PaymentStatus::Success)
    }

    async fn cancel_payment(&self, provider_reference: &str) -> Result<(), AppError> {
        // TODO: Implement cancellation
        Ok(())
    }

    async fn refund_payment(&self, provider_reference: &str, amount: Option<Decimal>) -> Result<PaymentResponse, AppError> {
        // TODO: Implement refund
        Ok(PaymentResponse {
            transaction_id: Uuid::new_v4(),
            provider_reference: format!("OM-REF-{}", Uuid::new_v4()),
            status: PaymentStatus::Refunded,
            amount: amount.unwrap_or(Decimal::ZERO),
            currency: "XOF".to_string(),
            fee: None,
            timestamp: Utc::now(),
            receipt_url: None,
            error_message: None,
        })
    }

    async fn verify_webhook(&self, webhook: &PaymentWebhook) -> Result<bool, AppError> {
        // TODO: Implement signature verification
        Ok(true)
    }
}

struct WaveProvider {
    api_url: String,
    api_key: String,
}

impl WaveProvider {
    fn new() -> Self {
        Self {
            api_url: std::env::var("WAVE_API_URL")
                .unwrap_or_else(|_| "https://api.wave.com/v1".to_string()),
            api_key: std::env::var("WAVE_API_KEY").unwrap_or_default(),
        }
    }
}

#[async_trait]
impl PaymentProviderTrait for WaveProvider {
    async fn initiate_payment(&self, request: &PaymentRequest) -> Result<PaymentResponse, AppError> {
        // TODO: Implement actual Wave API call
        info!("Wave payment initiated for amount {}", request.amount);
        
        Ok(PaymentResponse {
            transaction_id: request.transaction_id,
            provider_reference: format!("WAVE-{}", Uuid::new_v4()),
            status: PaymentStatus::Pending,
            amount: request.amount,
            currency: request.currency.clone(),
            fee: Some(request.amount * Decimal::new(1, 2)), // 1% fee
            timestamp: Utc::now(),
            receipt_url: None,
            error_message: None,
        })
    }

    async fn check_status(&self, provider_reference: &str) -> Result<PaymentStatus, AppError> {
        Ok(PaymentStatus::Success)
    }

    async fn cancel_payment(&self, provider_reference: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn refund_payment(&self, provider_reference: &str, amount: Option<Decimal>) -> Result<PaymentResponse, AppError> {
        Ok(PaymentResponse {
            transaction_id: Uuid::new_v4(),
            provider_reference: format!("WAVE-REF-{}", Uuid::new_v4()),
            status: PaymentStatus::Refunded,
            amount: amount.unwrap_or(Decimal::ZERO),
            currency: "XOF".to_string(),
            fee: None,
            timestamp: Utc::now(),
            receipt_url: None,
            error_message: None,
        })
    }

    async fn verify_webhook(&self, webhook: &PaymentWebhook) -> Result<bool, AppError> {
        Ok(true)
    }
}

struct MtnMoneyProvider {
    api_url: String,
    subscription_key: String,
}

impl MtnMoneyProvider {
    fn new() -> Self {
        Self {
            api_url: std::env::var("MTN_MOMO_API_URL")
                .unwrap_or_else(|_| "https://proxy.momoapi.mtn.com".to_string()),
            subscription_key: std::env::var("MTN_MOMO_SUBSCRIPTION_KEY").unwrap_or_default(),
        }
    }
}

#[async_trait]
impl PaymentProviderTrait for MtnMoneyProvider {
    async fn initiate_payment(&self, request: &PaymentRequest) -> Result<PaymentResponse, AppError> {
        info!("MTN Money payment initiated for amount {}", request.amount);
        
        Ok(PaymentResponse {
            transaction_id: request.transaction_id,
            provider_reference: format!("MTN-{}", Uuid::new_v4()),
            status: PaymentStatus::Pending,
            amount: request.amount,
            currency: request.currency.clone(),
            fee: Some(request.amount * Decimal::new(2, 2)), // 2% fee
            timestamp: Utc::now(),
            receipt_url: None,
            error_message: None,
        })
    }

    async fn check_status(&self, provider_reference: &str) -> Result<PaymentStatus, AppError> {
        Ok(PaymentStatus::Success)
    }

    async fn cancel_payment(&self, provider_reference: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn refund_payment(&self, provider_reference: &str, amount: Option<Decimal>) -> Result<PaymentResponse, AppError> {
        Ok(PaymentResponse {
            transaction_id: Uuid::new_v4(),
            provider_reference: format!("MTN-REF-{}", Uuid::new_v4()),
            status: PaymentStatus::Refunded,
            amount: amount.unwrap_or(Decimal::ZERO),
            currency: "XOF".to_string(),
            fee: None,
            timestamp: Utc::now(),
            receipt_url: None,
            error_message: None,
        })
    }

    async fn verify_webhook(&self, webhook: &PaymentWebhook) -> Result<bool, AppError> {
        Ok(true)
    }
}

struct CashProvider;

impl CashProvider {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PaymentProviderTrait for CashProvider {
    async fn initiate_payment(&self, request: &PaymentRequest) -> Result<PaymentResponse, AppError> {
        // Cash payments are marked as pending until manually confirmed
        Ok(PaymentResponse {
            transaction_id: request.transaction_id,
            provider_reference: format!("CASH-{}", Uuid::new_v4()),
            status: PaymentStatus::Pending,
            amount: request.amount,
            currency: request.currency.clone(),
            fee: None,
            timestamp: Utc::now(),
            receipt_url: None,
            error_message: None,
        })
    }

    async fn check_status(&self, provider_reference: &str) -> Result<PaymentStatus, AppError> {
        // Cash status must be manually updated
        Ok(PaymentStatus::Pending)
    }

    async fn cancel_payment(&self, provider_reference: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn refund_payment(&self, provider_reference: &str, amount: Option<Decimal>) -> Result<PaymentResponse, AppError> {
        // Cash refunds must be handled manually
        Ok(PaymentResponse {
            transaction_id: Uuid::new_v4(),
            provider_reference: format!("CASH-REF-{}", Uuid::new_v4()),
            status: PaymentStatus::Pending,
            amount: amount.unwrap_or(Decimal::ZERO),
            currency: "XOF".to_string(),
            fee: None,
            timestamp: Utc::now(),
            receipt_url: None,
            error_message: None,
        })
    }

    async fn verify_webhook(&self, webhook: &PaymentWebhook) -> Result<bool, AppError> {
        // No webhooks for cash
        Ok(false)
    }
}