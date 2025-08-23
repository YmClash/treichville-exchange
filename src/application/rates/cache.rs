use async_trait::async_trait;
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::{
    domain::rate::{ExchangeRate, PublicRate},
    shared::errors::AppError,
};

const RATE_CACHE_TTL: u64 = 30; // 30 seconds
const PUBLIC_RATES_KEY: &str = "rates:public";
const BEST_RATES_PREFIX: &str = "rates:best:";
const CHANGEUR_RATES_PREFIX: &str = "rates:changeur:";
const RATE_PAIR_PREFIX: &str = "rates:pair:";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRate {
    pub data: ExchangeRate,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPublicRates {
    pub data: Vec<PublicRate>,
    pub cached_at: DateTime<Utc>,
}

#[async_trait]
pub trait RateCacheTrait: Send + Sync {
    async fn get_public_rates(&self) -> Result<Option<Vec<PublicRate>>, AppError>;
    async fn set_public_rates(&self, rates: Vec<PublicRate>) -> Result<(), AppError>;
    async fn get_rates_by_pair(&self, from: &str, to: &str) -> Result<Option<Vec<ExchangeRate>>, AppError>;
    async fn set_rates_by_pair(&self, from: &str, to: &str, rates: Vec<ExchangeRate>) -> Result<(), AppError>;
    async fn get_changeur_rates(&self, changeur_id: Uuid) -> Result<Option<Vec<ExchangeRate>>, AppError>;
    async fn set_changeur_rates(&self, changeur_id: Uuid, rates: Vec<ExchangeRate>) -> Result<(), AppError>;
    async fn invalidate_changeur_rates(&self, changeur_id: Uuid) -> Result<(), AppError>;
    async fn invalidate_pair_rates(&self, from: &str, to: &str) -> Result<(), AppError>;
    async fn invalidate_all(&self) -> Result<(), AppError>;
    async fn get_best_rates(&self, from: &str, to: &str, amount: Decimal) -> Result<Option<Vec<ExchangeRate>>, AppError>;
    async fn set_best_rates(&self, from: &str, to: &str, amount: Decimal, rates: Vec<ExchangeRate>) -> Result<(), AppError>;
}

pub struct RateCache {
    redis: redis::aio::ConnectionManager,
}

impl RateCache {
    pub fn new(redis: redis::aio::ConnectionManager) -> Self {
        Self { redis }
    }

    fn get_pair_key(from: &str, to: &str) -> String {
        format!("{}{}:{}", RATE_PAIR_PREFIX, from.to_uppercase(), to.to_uppercase())
    }

    fn get_changeur_key(changeur_id: Uuid) -> String {
        format!("{}{}", CHANGEUR_RATES_PREFIX, changeur_id)
    }

    fn get_best_rates_key(from: &str, to: &str, amount: Decimal) -> String {
        format!("{}{}:{}:{}", BEST_RATES_PREFIX, from.to_uppercase(), to.to_uppercase(), amount)
    }
}

#[async_trait]
impl RateCacheTrait for RateCache {
    async fn get_public_rates(&self) -> Result<Option<Vec<PublicRate>>, AppError> {
        let mut conn = self.redis.clone();
        let cached: Option<String> = conn
            .get(PUBLIC_RATES_KEY)
            .await
            .map_err(|e| {
                tracing::warn!("Redis get error: {}", e);
                AppError::service_unavailable(format!("Redis error: {}", e))
            })?;

        if let Some(data) = cached {
            let cached_rates: CachedPublicRates = serde_json::from_str(&data)?;
            
            // Check if cache is still valid (30 seconds)
            let age = Utc::now() - cached_rates.cached_at;
            if age.num_seconds() <= RATE_CACHE_TTL as i64 {
                return Ok(Some(cached_rates.data));
            }
        }

        Ok(None)
    }

    async fn set_public_rates(&self, rates: Vec<PublicRate>) -> Result<(), AppError> {
        let cached = CachedPublicRates {
            data: rates,
            cached_at: Utc::now(),
        };

        let serialized = serde_json::to_string(&cached)?;
        
        let mut conn = self.redis.clone();
        let _: () = conn
            .set_ex(PUBLIC_RATES_KEY, serialized, RATE_CACHE_TTL as u64)
            .await
            .map_err(|e| {
                tracing::warn!("Redis set error: {}", e);
                AppError::service_unavailable(format!("Redis error: {}", e))
            })?;

        Ok(())
    }

    async fn get_rates_by_pair(&self, from: &str, to: &str) -> Result<Option<Vec<ExchangeRate>>, AppError> {
        let key = Self::get_pair_key(from, to);
        
        let cached: Option<String> = self.redis.clone()
            .get(&key)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

        if let Some(data) = cached {
            let rates: Vec<ExchangeRate> = serde_json::from_str(&data)?;
            return Ok(Some(rates));
        }

        Ok(None)
    }

    async fn set_rates_by_pair(&self, from: &str, to: &str, rates: Vec<ExchangeRate>) -> Result<(), AppError> {
        let key = Self::get_pair_key(from, to);
        let serialized = serde_json::to_string(&rates)?;
        
        let _: () = self.redis.clone()
            .set_ex(key, serialized, RATE_CACHE_TTL as u64)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

        Ok(())
    }

    async fn get_changeur_rates(&self, changeur_id: Uuid) -> Result<Option<Vec<ExchangeRate>>, AppError> {
        let key = Self::get_changeur_key(changeur_id);
        
        let cached: Option<String> = self.redis.clone()
            .get(&key)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

        if let Some(data) = cached {
            let rates: Vec<ExchangeRate> = serde_json::from_str(&data)?;
            return Ok(Some(rates));
        }

        Ok(None)
    }

    async fn set_changeur_rates(&self, changeur_id: Uuid, rates: Vec<ExchangeRate>) -> Result<(), AppError> {
        let key = Self::get_changeur_key(changeur_id);
        let serialized = serde_json::to_string(&rates)?;
        
        let _: () = self.redis.clone()
            .set_ex(key, serialized, RATE_CACHE_TTL as u64)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

        Ok(())
    }

    async fn invalidate_changeur_rates(&self, changeur_id: Uuid) -> Result<(), AppError> {
        let key = Self::get_changeur_key(changeur_id);
        
        let _: () = self.redis.clone()
            .del(&key)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

        // Also invalidate public rates as they might have changed
        let _: () = self.redis.clone()
            .del(PUBLIC_RATES_KEY)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

        Ok(())
    }

    async fn invalidate_pair_rates(&self, from: &str, to: &str) -> Result<(), AppError> {
        let key = Self::get_pair_key(from, to);
        
        let _: () = self.redis.clone()
            .del(&key)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

        Ok(())
    }

    async fn invalidate_all(&self) -> Result<(), AppError> {
        // Delete all rate-related keys
        let patterns = vec![
            format!("{}*", RATE_PAIR_PREFIX),
            format!("{}*", CHANGEUR_RATES_PREFIX),
            format!("{}*", BEST_RATES_PREFIX),
            PUBLIC_RATES_KEY.to_string(),
        ];

        for pattern in patterns {
            let keys: Vec<String> = self.redis.clone()
                .keys(&pattern)
                .await
                .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

            if !keys.is_empty() {
                let _: () = self.redis.clone()
                    .del(keys)
                    .await
                    .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;
            }
        }

        Ok(())
    }

    async fn get_best_rates(&self, from: &str, to: &str, amount: Decimal) -> Result<Option<Vec<ExchangeRate>>, AppError> {
        let key = format!("{}{}:{}:{}", BEST_RATES_PREFIX, from, to, amount);
        
        let result: Option<String> = self.redis.clone()
            .get(&key)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;
        
        match result {
            Some(data) => {
                let rates = serde_json::from_str(&data)
                    .map_err(|e| AppError::service_unavailable(format!("Deserialization error: {}", e)))?;
                Ok(Some(rates))
            },
            None => Ok(None)
        }
    }
    
    async fn set_best_rates(&self, from: &str, to: &str, amount: Decimal, rates: Vec<ExchangeRate>) -> Result<(), AppError> {
        let key = format!("{}{}:{}:{}", BEST_RATES_PREFIX, from, to, amount);
        let serialized = serde_json::to_string(&rates)
            .map_err(|e| AppError::service_unavailable(format!("Serialization error: {}", e)))?;
        
        let _: () = self.redis.clone()
            .set_ex(&key, serialized, RATE_CACHE_TTL as u64)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;
        
        Ok(())
    }
}

// Additional cache methods for best rates
impl RateCache {
    pub async fn get_best_rates(
        &self, 
        from: &str, 
        to: &str, 
        amount: Decimal
    ) -> Result<Option<Vec<ExchangeRate>>, AppError> {
        let key = Self::get_best_rates_key(from, to, amount);
        
        let cached: Option<String> = self.redis.clone()
            .get(&key)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

        if let Some(data) = cached {
            let rates: Vec<ExchangeRate> = serde_json::from_str(&data)?;
            return Ok(Some(rates));
        }

        Ok(None)
    }

    pub async fn set_best_rates(
        &self,
        from: &str,
        to: &str,
        amount: Decimal,
        rates: Vec<ExchangeRate>
    ) -> Result<(), AppError> {
        let key = Self::get_best_rates_key(from, to, amount);
        let serialized = serde_json::to_string(&rates)?;
        
        let _: () = self.redis.clone()
            .set_ex(key, serialized, RATE_CACHE_TTL as u64)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

        Ok(())
    }

    // Pub/Sub for real-time rate updates
    pub async fn publish_rate_update(&self, rate: &ExchangeRate) -> Result<(), AppError> {
        let channel = format!("rates:updates:{}:{}", rate.from_currency, rate.to_currency);
        let message = serde_json::to_string(rate)?;
        
        let _: () = self.redis.clone()
            .publish(channel, message)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

        Ok(())
    }
}