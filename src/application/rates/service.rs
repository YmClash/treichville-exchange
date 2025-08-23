use std::sync::Arc;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use thiserror::Error;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::{
    application::rates::{
        aggregator::{RateAggregator, AggregatedRate, MarketDepth, ChangeurRanking},
        cache::{RateCache, RateCacheTrait},
        repository::{RateRepository, RateRepositoryTrait},
    },
    domain::{
        rate::{
            ExchangeRate, PublicRate, RateQuote, CreateRateDto, UpdateRateDto,
            GetQuoteRequest, ExchangeOperation, BestRateResponse,
        },
        ChangeurProfile,
    },
    shared::errors::AppError,
};

#[derive(Debug, Error)]
pub enum RateServiceError {
    #[error("Rate not found")]
    NotFound,
    
    #[error("Invalid currency pair: {0}")]
    InvalidCurrencyPair(String),
    
    #[error("Insufficient liquidity for amount: {0}")]
    InsufficientLiquidity(Decimal),
    
    #[error("Rate manipulation detected")]
    ManipulationDetected,
    
    #[error("No rates available for this currency pair")]
    NoRatesAvailable,
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Cache error: {0}")]
    Cache(String),
}

impl From<RateServiceError> for AppError {
    fn from(err: RateServiceError) -> Self {
        match err {
            RateServiceError::NotFound => AppError::not_found("Rate"),
            RateServiceError::InvalidCurrencyPair(msg) => AppError::validation(msg),
            RateServiceError::InsufficientLiquidity(amount) => {
                AppError::bad_request(format!("Insufficient liquidity for amount: {}", amount))
            }
            RateServiceError::ManipulationDetected => {
                AppError::forbidden("Rate manipulation detected")
            }
            RateServiceError::NoRatesAvailable => {
                AppError::not_found("No rates available for this currency pair")
            }
            RateServiceError::Database(e) => AppError::from(e),
            RateServiceError::Cache(msg) => AppError::service_unavailable(msg),
        }
    }
}

pub struct RateService {
    repository: Arc<dyn RateRepositoryTrait>,
    cache: Arc<dyn RateCacheTrait>,
    fee_percentage: Decimal,
}

impl RateService {
    pub fn new(
        repository: Arc<dyn RateRepositoryTrait>,
        cache: Arc<dyn RateCacheTrait>,
        fee_percentage: Decimal,
    ) -> Self {
        Self {
            repository,
            cache,
            fee_percentage,
        }
    }

    pub fn with_defaults(
        repository: RateRepository,
        cache: RateCache,
    ) -> Self {
        Self {
            repository: Arc::new(repository),
            cache: Arc::new(cache),
            fee_percentage: Decimal::new(5, 3), // 0.5%
        }
    }

    // CRUD Operations
    pub async fn create_rate(
        &self,
        changeur_id: Uuid,
        dto: CreateRateDto,
    ) -> Result<ExchangeRate, AppError> {
        // Validate the rate is not manipulated
        let is_manipulated = self.repository
            .detect_manipulation(
                &dto.from_currency,
                &dto.to_currency,
                dto.sell_rate,
            )
            .await?;

        if is_manipulated {
            warn!(
                changeur_id = %changeur_id,
                from = dto.from_currency,
                to = dto.to_currency,
                rate = %dto.sell_rate,
                "Rate manipulation detected"
            );
            return Err(RateServiceError::ManipulationDetected.into());
        }

        // Create the rate
        let rate = self.repository.create(changeur_id, dto).await?;

        // Invalidate cache
        self.cache.invalidate_changeur_rates(changeur_id).await?;
        self.cache.invalidate_pair_rates(&rate.from_currency, &rate.to_currency).await?;

        info!(
            changeur_id = %changeur_id,
            from = rate.from_currency,
            to = rate.to_currency,
            buy_rate = %rate.buy_rate,
            sell_rate = %rate.sell_rate,
            "New rate created"
        );

        Ok(rate)
    }

    pub async fn update_rate(
        &self,
        id: Uuid,
        changeur_id: Uuid,
        dto: UpdateRateDto,
    ) -> Result<ExchangeRate, AppError> {
        // Update the rate
        let rate = self.repository.update(id, changeur_id, dto).await?;

        // Invalidate cache
        self.cache.invalidate_changeur_rates(changeur_id).await?;
        self.cache.invalidate_pair_rates(&rate.from_currency, &rate.to_currency).await?;

        info!(
            rate_id = %id,
            changeur_id = %changeur_id,
            "Rate updated"
        );

        Ok(rate)
    }

    pub async fn delete_rate(
        &self,
        id: Uuid,
        changeur_id: Uuid,
    ) -> Result<(), AppError> {
        // Get rate details before deletion for cache invalidation
        let rate = self.repository.find_by_id(id).await?
            .ok_or(RateServiceError::NotFound)?;

        // Delete (soft delete - mark as inactive)
        self.repository.delete(id, changeur_id).await?;

        // Invalidate cache
        self.cache.invalidate_changeur_rates(changeur_id).await?;
        self.cache.invalidate_pair_rates(&rate.from_currency, &rate.to_currency).await?;

        info!(
            rate_id = %id,
            changeur_id = %changeur_id,
            "Rate deleted"
        );

        Ok(())
    }

    // Query Operations
    pub async fn get_public_rates(&self) -> Result<Vec<PublicRate>, AppError> {
        // Try cache first
        if let Some(cached) = self.cache.get_public_rates().await? {
            return Ok(cached);
        }

        // Fetch from database
        let rates = self.repository.get_public_rates().await?;

        // Cache the results
        self.cache.set_public_rates(rates.clone()).await?;

        Ok(rates)
    }

    pub async fn get_rates_by_pair(
        &self,
        from: &str,
        to: &str,
    ) -> Result<Vec<ExchangeRate>, AppError> {
        // Try cache first
        if let Some(cached) = self.cache.get_rates_by_pair(from, to).await? {
            return Ok(cached);
        }

        // Fetch from database
        let rates = self.repository.find_active_by_pair(from, to).await?;

        if rates.is_empty() {
            return Err(RateServiceError::NoRatesAvailable.into());
        }

        // Cache the results
        self.cache.set_rates_by_pair(from, to, rates.clone()).await?;

        Ok(rates)
    }

    pub async fn get_changeur_rates(&self, changeur_id: Uuid) -> Result<Vec<ExchangeRate>, AppError> {
        // Try cache first
        if let Some(cached) = self.cache.get_changeur_rates(changeur_id).await? {
            return Ok(cached);
        }

        // Fetch from database
        let rates = self.repository.find_by_changeur(changeur_id).await?;

        // Cache the results
        self.cache.set_changeur_rates(changeur_id, rates.clone()).await?;

        Ok(rates)
    }

    // Quote and Best Rate Operations
    pub async fn get_quote(
        &self,
        request: GetQuoteRequest,
    ) -> Result<RateQuote, AppError> {
        request.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;

        let rates = self.repository.get_best_rates(
            &request.from_currency,
            &request.to_currency,
            request.amount,
        ).await?;

        if rates.is_empty() {
            return Err(RateServiceError::NoRatesAvailable.into());
        }

        let best_rate = &rates[0];
        let is_buying = matches!(request.operation, ExchangeOperation::Buy);

        // TODO: Get changeur name from database
        let changeur_name = "Changeur Name".to_string();

        let quote = RateQuote::new(
            best_rate,
            request.amount,
            is_buying,
            self.fee_percentage,
            changeur_name,
        );

        Ok(quote)
    }

    pub async fn get_best_rates(
        &self,
        from: &str,
        to: &str,
        amount: Decimal,
    ) -> Result<BestRateResponse, AppError> {
        // Check cache first
        if let Ok(Some(cached)) = self.cache.get_best_rates(from, to, amount).await {
            return self.build_best_rate_response(cached, amount).await;
        }

        // Fetch from database
        let rates = self.repository.get_best_rates(from, to, amount).await?;

        if rates.is_empty() {
            return Err(RateServiceError::NoRatesAvailable.into());
        }

        // Cache the results
        let _ = self.cache.set_best_rates(from, to, amount, rates.clone()).await;

        self.build_best_rate_response(rates, amount).await
    }

    async fn build_best_rate_response(
        &self,
        rates: Vec<ExchangeRate>,
        amount: Decimal,
    ) -> Result<BestRateResponse, AppError> {
        let mut quotes = vec![];

        for rate in &rates {
            // TODO: Get changeur name from database
            let changeur_name = format!("Changeur {}", rate.changeur_id);
            
            let quote = RateQuote::new(
                rate,
                amount,
                false, // Assuming selling
                self.fee_percentage,
                changeur_name,
            );
            quotes.push(quote);
        }

        let best_quote = quotes[0].clone();

        // Calculate average rate for savings calculation
        let all_rates = self.repository
            .find_active_by_pair(&rates[0].from_currency, &rates[0].to_currency)
            .await?;
        
        let average_rate = if !all_rates.is_empty() {
            all_rates.iter().map(|r| r.sell_rate).sum::<Decimal>() / Decimal::from(all_rates.len())
        } else {
            best_quote.rate
        };

        let savings = RateAggregator::calculate_savings(
            best_quote.rate,
            average_rate,
            amount,
            false,
        );

        Ok(BestRateResponse {
            best_quote,
            quotes,
            savings_vs_average: savings,
        })
    }

    // Aggregation Operations
    pub async fn get_aggregated_rates(
        &self,
        from: &str,
        to: &str,
    ) -> Result<AggregatedRate, AppError> {
        let rates = self.get_rates_by_pair(from, to).await?;
        RateAggregator::aggregate_rates(&rates).map_err(Into::into)
    }

    pub async fn get_market_depth(
        &self,
        from: &str,
        to: &str,
    ) -> Result<MarketDepth, AppError> {
        let rates = self.get_rates_by_pair(from, to).await?;
        
        // TODO: Fetch changeur names from database
        let changeur_names = std::collections::HashMap::new();
        
        Ok(RateAggregator::build_market_depth(&rates, changeur_names))
    }

    pub async fn rank_changeurs(
        &self,
        from: &str,
        to: &str,
    ) -> Result<Vec<ChangeurRanking>, AppError> {
        let rates = self.get_rates_by_pair(from, to).await?;
        Ok(RateAggregator::rank_changeurs_by_competitiveness(&rates))
    }

    pub async fn detect_outliers(
        &self,
        from: &str,
        to: &str,
    ) -> Result<Vec<Uuid>, AppError> {
        let rates = self.get_rates_by_pair(from, to).await?;
        Ok(RateAggregator::detect_outliers(&rates))
    }

    // Bulk Operations
    pub async fn bulk_update_rates(
        &self,
        changeur_id: Uuid,
        updates: Vec<(String, String, Decimal, Decimal)>, // (from, to, buy, sell)
    ) -> Result<Vec<ExchangeRate>, AppError> {
        let mut updated_rates = vec![];

        for (from, to, buy_rate, sell_rate) in updates {
            // Find existing rate
            let existing = self.repository
                .find_active_by_pair(&from, &to)
                .await?
                .into_iter()
                .find(|r| r.changeur_id == changeur_id);

            let rate = if let Some(existing_rate) = existing {
                // Update existing
                let dto = UpdateRateDto {
                    buy_rate: Some(buy_rate),
                    sell_rate: Some(sell_rate),
                    available_amount: None,
                    min_amount: None,
                    max_amount: None,
                    is_active: None,
                };
                self.repository.update(existing_rate.id, changeur_id, dto).await?
            } else {
                // Create new
                let dto = CreateRateDto {
                    from_currency: from,
                    to_currency: to,
                    buy_rate,
                    sell_rate,
                    available_amount: None,
                    min_amount: None,
                    max_amount: None,
                };
                self.repository.create(changeur_id, dto).await?
            };

            updated_rates.push(rate);
        }

        // Invalidate all cache
        self.cache.invalidate_changeur_rates(changeur_id).await?;
        self.cache.invalidate_all().await?;

        info!(
            changeur_id = %changeur_id,
            count = updated_rates.len(),
            "Bulk rate update completed"
        );

        Ok(updated_rates)
    }

    // Analytics
    pub async fn get_rate_statistics(
        &self,
        from: &str,
        to: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<RateStatistics, AppError> {
        let history = self.repository.get_rate_history(from, to, start, end).await?;
        
        if history.is_empty() {
            return Err(AppError::not_found("No historical data available"));
        }

        let buy_rates: Vec<Decimal> = history.iter().map(|h| h.buy_rate).collect();
        let sell_rates: Vec<Decimal> = history.iter().map(|h| h.sell_rate).collect();

        let stats = RateStatistics {
            period_start: start,
            period_end: end,
            min_buy_rate: *buy_rates.iter().min()
                .ok_or_else(|| AppError::InternalServerError)?,
            max_buy_rate: *buy_rates.iter().max()
                .ok_or_else(|| AppError::InternalServerError)?,
            avg_buy_rate: buy_rates.iter().sum::<Decimal>() / Decimal::from(buy_rates.len()),
            min_sell_rate: *sell_rates.iter().min()
                .ok_or_else(|| AppError::InternalServerError)?,
            max_sell_rate: *sell_rates.iter().max()
                .ok_or_else(|| AppError::InternalServerError)?,
            avg_sell_rate: sell_rates.iter().sum::<Decimal>() / Decimal::from(sell_rates.len()),
            volatility_buy: RateAggregator::calculate_std_deviation(&buy_rates),
            volatility_sell: RateAggregator::calculate_std_deviation(&sell_rates),
            data_points: history.len(),
        };

        Ok(stats)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateStatistics {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub min_buy_rate: Decimal,
    pub max_buy_rate: Decimal,
    pub avg_buy_rate: Decimal,
    pub min_sell_rate: Decimal,
    pub max_sell_rate: Decimal,
    pub avg_sell_rate: Decimal,
    pub volatility_buy: Decimal,
    pub volatility_sell: Decimal,
    pub data_points: usize,
}

use serde::{Deserialize, Serialize};