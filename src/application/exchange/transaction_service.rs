use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, error};
use uuid::Uuid;
use validator::Validate;

use crate::{
    application::{
        auth::{UserRepository, UserRepositoryTrait},
        exchange::{
            matching_engine::{MatchingEngine, MatchingCriteria, UrgencyLevel, MatchResult},
            payment_processor::{PaymentProcessor, PaymentRequest, PaymentProvider, PaymentStatus},
            transaction_repository::{TransactionRepository, TransactionRepositoryTrait, DatePeriod},
            wallet_service::{WalletService, WalletServiceTrait},
        },
        rates::{RateService, RateServiceError},
    },
    domain::{
        rate::{GetQuoteRequest, ExchangeOperation},
        transaction::{Transaction, TransactionStatus, TransactionType, PaymentMethod},
        user::User,
    },
    shared::{errors::AppError, utils},
};

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateTransactionDto {
    #[validate(custom = "utils::validate_currency_code")]
    pub from_currency: String,
    
    #[validate(custom = "utils::validate_currency_code")]
    pub to_currency: String,
    
    pub amount: Decimal,
    
    pub payment_method: PaymentMethod,
    pub preferred_changeur_id: Option<Uuid>,
    pub urgency_level: Option<UrgencyLevel>,
    pub location: Option<LocationDto>,
    pub notes: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationDto {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ConfirmPaymentDto {
    pub transaction_id: Uuid,
    pub payment_reference: String,
    pub payment_proof: Option<serde_json::Value>,
    #[validate(custom = "utils::validate_phone_number")]
    pub sender_phone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub id: Uuid,
    pub reference: String,
    pub status: TransactionStatus,
    pub from_currency: String,
    pub to_currency: String,
    pub amount_from: Decimal,
    pub amount_to: Decimal,
    pub rate_applied: Decimal,
    pub fee: Decimal,
    pub total_amount: Decimal,
    pub payment_method: PaymentMethod,
    pub changeur: Option<ChangeurInfo>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeurInfo {
    pub id: Uuid,
    pub name: String,
    pub phone: Option<String>,
    pub rating: Option<Decimal>,
    pub distance_km: Option<f64>,
}

pub struct TransactionService {
    repository: Arc<dyn TransactionRepositoryTrait>,
    user_repository: Arc<dyn UserRepositoryTrait>,
    rate_service: Arc<RateService>,
    matching_engine: Arc<tokio::sync::Mutex<MatchingEngine>>,
    wallet_service: Arc<dyn WalletServiceTrait>,
    payment_processor: Arc<PaymentProcessor>,
    settings: crate::shared::config::Settings,
}

impl TransactionService {
    pub fn new(
        repository: Arc<dyn TransactionRepositoryTrait>,
        user_repository: Arc<dyn UserRepositoryTrait>,
        rate_service: Arc<RateService>,
        wallet_service: Arc<dyn WalletServiceTrait>,
        payment_processor: Arc<PaymentProcessor>,
        settings: crate::shared::config::Settings,
    ) -> Self {
        Self {
            repository,
            user_repository,
            rate_service,
            matching_engine: Arc::new(tokio::sync::Mutex::new(MatchingEngine::new())),
            wallet_service,
            payment_processor,
            settings,
        }
    }

    pub async fn create_transaction(
        &self,
        user_id: Uuid,
        dto: CreateTransactionDto,
    ) -> Result<TransactionResponse, AppError> {
        dto.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;

        // Check idempotency
        if let Some(ref key) = dto.idempotency_key {
            if let Some(existing) = self.repository.find_by_idempotency_key(key).await? {
                info!(
                    idempotency_key = key,
                    transaction_id = %existing.id,
                    "Idempotent request, returning existing transaction"
                );
                return self.build_response(existing).await;
            }
        }

        // Check user limits
        self.check_user_limits(user_id, dto.amount).await?;

        // Get best rates
        let rates = self.rate_service
            .get_best_rates(&dto.from_currency, &dto.to_currency, dto.amount)
            .await?;

        if rates.quotes.is_empty() {
            return Err(AppError::not_found("No rates available for this currency pair"));
        }

        // Prepare matching criteria
        let criteria = MatchingCriteria {
            from_currency: dto.from_currency.clone(),
            to_currency: dto.to_currency.clone(),
            amount: dto.amount,
            client_id: user_id,
            preferred_changeur_id: dto.preferred_changeur_id,
            max_rate: None,
            min_rating: Some(Decimal::from(3)),
            location_radius_km: dto.location.as_ref().map(|_| 10.0),
            client_latitude: dto.location.as_ref().map(|l| l.latitude),
            client_longitude: dto.location.as_ref().map(|l| l.longitude),
            urgency_level: dto.urgency_level.unwrap_or(UrgencyLevel::Normal),
        };

        // Find best match
        let available_rates = self.rate_service
            .get_rates_by_pair(&dto.from_currency, &dto.to_currency)
            .await?;

        // Get changeur profiles for each rate
        let mut changeur_profiles = vec![];
        for rate in &available_rates {
            if let Some(profile) = self.user_repository.get_changeur_profile(rate.changeur_id).await? {
                changeur_profiles.push(profile);
            }
        }

        let match_result = {
            let matching_engine = self.matching_engine.lock().await;
            matching_engine.find_best_match(&criteria, available_rates, changeur_profiles).await?
        };

        // Calculate amounts
        let best_quote = &rates.best_quote;
        let amount_to = best_quote.amount_to;
        let fee = best_quote.fee;
        let total_amount = best_quote.total;

        // Create transaction
        let transaction = Transaction::new(
            user_id,
            Some(match_result.changeur_id),
            TransactionType::Exchange,
            dto.from_currency.clone(),
            dto.to_currency.clone(),
            dto.amount,
            amount_to,
            match_result.rate.sell_rate,
            fee,
            dto.payment_method,
        );

        // Save transaction with idempotency key
        let mut saved_transaction = transaction.clone();
        saved_transaction.idempotency_key = dto.idempotency_key;
        saved_transaction.notes = dto.notes;
        saved_transaction = self.repository.create(saved_transaction).await?;

        // Reserve changeur
        {
            let mut matching_engine = self.matching_engine.lock().await;
            matching_engine.reserve_changeur(
                saved_transaction.id,
                match_result.changeur_id,
                dto.amount,
                dto.from_currency.clone(),
            )?;
        }

        // For changeurs with wallets, reserve funds
        if dto.payment_method != PaymentMethod::Cash {
            info!(
                "Reserving funds for electronic payment: changeur_id={}, currency={}, amount={}",
                match_result.changeur_id, dto.to_currency, amount_to
            );
            
            match self.wallet_service.reserve(
                match_result.changeur_id,
                &dto.to_currency,
                amount_to,
                &saved_transaction.reference,
            ).await {
                Ok(wallet) => {
                    info!("Funds reserved successfully: available={}, reserved={}", 
                          wallet.balance, wallet.reserved_balance);
                }
                Err(e) => {
                    error!("Failed to reserve funds: {}", e);
                    return Err(e);
                }
            }
        } else {
            info!("Cash payment, no wallet reservation needed");
        }

        info!(
            transaction_id = %saved_transaction.id,
            reference = saved_transaction.reference,
            amount = %dto.amount,
            "Transaction created successfully"
        );

        self.build_response(saved_transaction).await
    }

    pub async fn confirm_payment(
        &self,
        user_id: Uuid,
        dto: ConfirmPaymentDto,
    ) -> Result<TransactionResponse, AppError> {
        dto.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;

        // Get transaction
        let mut transaction = self.repository
            .find_by_id(dto.transaction_id)
            .await?
            .ok_or_else(|| AppError::not_found("Transaction"))?;

        // Verify ownership
        if transaction.client_id != user_id {
            return Err(AppError::forbidden("Not authorized to confirm this transaction"));
        }

        // Check status
        if transaction.status != TransactionStatus::Pending {
            return Err(AppError::bad_request(
                format!("Transaction cannot be confirmed in status: {:?}", transaction.status)
            ));
        }

        // Check expiration
        if transaction.is_expired() {
            self.expire_transaction(transaction.id).await?;
            return Err(AppError::bad_request("Transaction has expired"));
        }

        // Process payment based on method
        let payment_response = match transaction.payment_method {
            PaymentMethod::Cash => {
                // Cash payments are confirmed manually
                self.repository.update_payment_proof(
                    transaction.id,
                    dto.payment_reference.clone(),
                    dto.payment_proof.unwrap_or(serde_json::json!({})),
                ).await?;
                
                self.repository.update_status(
                    transaction.id,
                    TransactionStatus::Paid,
                ).await?
            }
            _ => {
                // Process electronic payment
                let payment_request = PaymentRequest {
                    transaction_id: transaction.id,
                    provider: transaction.payment_method.clone().into(),
                    amount: transaction.total_amount,
                    currency: transaction.from_currency.clone(),
                    sender_phone: dto.sender_phone.unwrap_or_default(),
                    recipient_phone: None,
                    reference: transaction.reference.clone(),
                    description: format!("Exchange {} to {}", 
                        transaction.from_currency, 
                        transaction.to_currency
                    ),
                    callback_url: Some(format!("{}/webhooks/payment", self.settings.app.name)),
                    metadata: None,
                };

                let payment_response = self.payment_processor
                    .process_payment(payment_request)
                    .await?;

                if payment_response.status == PaymentStatus::Success {
                    self.repository.update_payment_proof(
                        transaction.id,
                        payment_response.provider_reference.clone(),
                        serde_json::json!(payment_response),
                    ).await?
                } else {
                    return Err(AppError::payment("Payment processing failed"));
                }
            }
        };

        info!(
            transaction_id = %transaction.id,
            reference = transaction.reference,
            "Payment confirmed"
        );

        self.build_response(payment_response).await
    }

    pub async fn complete_transaction(
        &self,
        transaction_id: Uuid,
        changeur_id: Uuid,
    ) -> Result<TransactionResponse, AppError> {
        // Get transaction
        let transaction = self.repository
            .find_by_id(transaction_id)
            .await?
            .ok_or_else(|| AppError::not_found("Transaction"))?;

        // Verify changeur
        if transaction.changeur_id != Some(changeur_id) {
            return Err(AppError::forbidden("Not authorized to complete this transaction"));
        }

        // Check status - Accept both Paid and Confirmed status
        if transaction.status != TransactionStatus::Paid && transaction.status != TransactionStatus::Confirmed {
            return Err(AppError::bad_request(
                format!("Transaction cannot be completed in status: {:?}", transaction.status)
            ));
        }

        // Transfer funds if using wallets
        if transaction.payment_method != PaymentMethod::Cash {
            // Release reserved funds
            self.wallet_service.release(
                changeur_id,
                &transaction.to_currency,
                transaction.amount_to,
                &transaction.reference,
            ).await?;

            // Transfer to client
            self.wallet_service.transfer(
                changeur_id,
                transaction.client_id,
                &transaction.to_currency,
                transaction.amount_to,
                &transaction.reference,
            ).await?;
        }

        // Update status
        let updated = self.repository
            .update_status(transaction_id, TransactionStatus::Completed)
            .await?;

        // Release changeur reservation
        {
            let mut matching_engine = self.matching_engine.lock().await;
            matching_engine.release_reservation(transaction_id);
        }

        info!(
            transaction_id = %transaction_id,
            reference = transaction.reference,
            "Transaction completed"
        );

        self.build_response(updated).await
    }

    pub async fn cancel_transaction(
        &self,
        transaction_id: Uuid,
        user_id: Uuid,
        reason: String,
    ) -> Result<TransactionResponse, AppError> {
        // Get transaction
        let mut transaction = self.repository
            .find_by_id(transaction_id)
            .await?
            .ok_or_else(|| AppError::not_found("Transaction"))?;

        // Verify ownership or admin
        if transaction.client_id != user_id && transaction.changeur_id != Some(user_id) {
            return Err(AppError::forbidden("Not authorized to cancel this transaction"));
        }

        // Check if cancellable
        if !transaction.can_be_cancelled() {
            return Err(AppError::bad_request(
                format!("Transaction cannot be cancelled in status: {:?}", transaction.status)
            ));
        }

        // Release reserved funds if any
        if let Some(changeur_id) = transaction.changeur_id {
            if transaction.payment_method != PaymentMethod::Cash {
                self.wallet_service.release(
                    changeur_id,
                    &transaction.to_currency,
                    transaction.amount_to,
                    &transaction.reference,
                ).await.ok(); // Don't fail if release fails
            }
        }

        // Update transaction
        transaction.cancel(reason.clone());
        let updated = self.repository
            .update_status(transaction_id, TransactionStatus::Cancelled)
            .await?;

        // Release changeur reservation
        {
            let mut matching_engine = self.matching_engine.lock().await;
            matching_engine.release_reservation(transaction_id);
        }

        info!(
            transaction_id = %transaction_id,
            reference = transaction.reference,
            reason = reason,
            "Transaction cancelled"
        );

        self.build_response(updated).await
    }

    pub async fn get_transaction(
        &self,
        transaction_id: Uuid,
        user_id: Uuid,
    ) -> Result<TransactionResponse, AppError> {
        let transaction = self.repository
            .find_by_id(transaction_id)
            .await?
            .ok_or_else(|| AppError::not_found("Transaction"))?;

        // Verify access
        if transaction.client_id != user_id && transaction.changeur_id != Some(user_id) {
            return Err(AppError::forbidden("Not authorized to view this transaction"));
        }

        self.build_response(transaction).await
    }

    pub async fn get_user_transactions(
        &self,
        user_id: Uuid,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<TransactionResponse>, AppError> {
        let limit = per_page.min(100) as i64;
        let offset = ((page.saturating_sub(1)) * per_page) as i64;

        let transactions = self.repository
            .find_by_user(user_id, limit, offset)
            .await?;

        let mut responses = vec![];
        for transaction in transactions {
            responses.push(self.build_response(transaction).await?);
        }

        Ok(responses)
    }

    async fn check_user_limits(&self, user_id: Uuid, amount: Decimal) -> Result<(), AppError> {
        // Get user volume for today and this month
        let daily_volume = self.repository
            .get_user_volume(user_id, DatePeriod::Today)
            .await?;
        
        let monthly_volume = self.repository
            .get_user_volume(user_id, DatePeriod::ThisMonth)
            .await?;

        // TODO: Get user KYC limits from database
        let daily_limit = Decimal::from(5000000); // 5M XOF
        let monthly_limit = Decimal::from(50000000); // 50M XOF

        if daily_volume.total_amount + amount > daily_limit {
            warn!(
                user_id = %user_id,
                daily_volume = %daily_volume.total_amount,
                requested = %amount,
                limit = %daily_limit,
                "Daily transaction limit exceeded"
            );
            return Err(AppError::KycLimitExceeded);
        }

        if monthly_volume.total_amount + amount > monthly_limit {
            warn!(
                user_id = %user_id,
                monthly_volume = %monthly_volume.total_amount,
                requested = %amount,
                limit = %monthly_limit,
                "Monthly transaction limit exceeded"
            );
            return Err(AppError::KycLimitExceeded);
        }

        Ok(())
    }

    async fn expire_transaction(&self, transaction_id: Uuid) -> Result<(), AppError> {
        // Release any reserved funds
        let transaction = self.repository
            .find_by_id(transaction_id)
            .await?
            .ok_or_else(|| AppError::not_found("Transaction"))?;

        if let Some(changeur_id) = transaction.changeur_id {
            if transaction.payment_method != PaymentMethod::Cash {
                self.wallet_service.release(
                    changeur_id,
                    &transaction.to_currency,
                    transaction.amount_to,
                    &transaction.reference,
                ).await.ok();
            }
        }

        // Update status
        self.repository
            .update_status(transaction_id, TransactionStatus::Expired)
            .await?;

        // Release changeur reservation
        {
            let mut matching_engine = self.matching_engine.lock().await;
            matching_engine.release_reservation(transaction_id);
        }

        Ok(())
    }

    pub async fn process_expired_transactions(&self) -> Result<u32, AppError> {
        let expired = self.repository.get_expired_transactions().await?;
        let count = expired.len() as u32;

        for transaction in expired {
            if let Err(e) = self.expire_transaction(transaction.id).await {
                error!(
                    transaction_id = %transaction.id,
                    error = %e,
                    "Failed to expire transaction"
                );
            }
        }

        info!(count = count, "Processed expired transactions");
        Ok(count)
    }

    async fn build_response(&self, transaction: Transaction) -> Result<TransactionResponse, AppError> {
        // TODO: Get changeur info from database
        let changeur_info = transaction.changeur_id.map(|id| ChangeurInfo {
            id,
            name: format!("Changeur {}", id),
            phone: None,
            rating: Some(dec!(4.5)),
            distance_km: None,
        });

        Ok(TransactionResponse {
            id: transaction.id,
            reference: transaction.reference,
            status: transaction.status,
            from_currency: transaction.from_currency,
            to_currency: transaction.to_currency,
            amount_from: transaction.amount_from,
            amount_to: transaction.amount_to,
            rate_applied: transaction.rate_applied,
            fee: transaction.fee,
            total_amount: transaction.total_amount,
            payment_method: transaction.payment_method,
            changeur: changeur_info,
            expires_at: transaction.expires_at,
            created_at: transaction.created_at,
        })
    }
}