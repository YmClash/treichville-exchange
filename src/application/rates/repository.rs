use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;

use crate::{
    domain::{
        rate::{ExchangeRate, PublicRate, CreateRateDto, UpdateRateDto},
        ChangeurProfile,
    },
    shared::errors::AppError,
};

#[async_trait]
pub trait RateRepositoryTrait: Send + Sync {
    async fn create(&self, changeur_id: Uuid, dto: CreateRateDto) -> Result<ExchangeRate, AppError>;
    async fn update(&self, id: Uuid, changeur_id: Uuid, dto: UpdateRateDto) -> Result<ExchangeRate, AppError>;
    async fn delete(&self, id: Uuid, changeur_id: Uuid) -> Result<(), AppError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<ExchangeRate>, AppError>;
    async fn find_by_changeur(&self, changeur_id: Uuid) -> Result<Vec<ExchangeRate>, AppError>;
    async fn find_active_by_pair(&self, from: &str, to: &str) -> Result<Vec<ExchangeRate>, AppError>;
    async fn get_public_rates(&self) -> Result<Vec<PublicRate>, AppError>;
    async fn get_best_rates(&self, from: &str, to: &str, amount: Decimal) -> Result<Vec<ExchangeRate>, AppError>;
    async fn record_rate_history(&self, rate: &ExchangeRate) -> Result<(), AppError>;
    async fn detect_manipulation(&self, from: &str, to: &str, rate: Decimal) -> Result<bool, AppError>;
    async fn get_rate_history(&self, from: &str, to: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<RateHistory>, AppError>;
}

pub struct RateRepository {
    db: Pool<Postgres>,
}

impl RateRepository {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RateRepositoryTrait for RateRepository {
    async fn create(&self, changeur_id: Uuid, dto: CreateRateDto) -> Result<ExchangeRate, AppError> {
        // Check if rate already exists for this changeur and currency pair
        let existing = sqlx::query(
            "SELECT id FROM exchange_rates 
             WHERE changeur_id = $1 AND from_currency = $2 AND to_currency = $3"
        )
        .bind(changeur_id)
        .bind(&dto.from_currency)
        .bind(&dto.to_currency)
        .fetch_optional(&self.db)
        .await?;

        if existing.is_some() {
            return Err(AppError::conflict("Rate already exists for this currency pair"));
        }

        let rate = ExchangeRate::new(
            changeur_id,
            dto.from_currency,
            dto.to_currency,
            dto.buy_rate,
            dto.sell_rate,
        );

        let saved_rate = sqlx::query_as::<_, ExchangeRate>(
            r#"
            INSERT INTO exchange_rates (
                id, changeur_id, from_currency, to_currency,
                buy_rate, sell_rate, mid_rate, spread,
                available_amount, min_amount, max_amount,
                is_active, last_update_source
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#
        )
        .bind(rate.id)
        .bind(rate.changeur_id)
        .bind(&rate.from_currency)
        .bind(&rate.to_currency)
        .bind(rate.buy_rate)
        .bind(rate.sell_rate)
        .bind(rate.mid_rate)
        .bind(rate.spread)
        .bind(dto.available_amount)
        .bind(dto.min_amount.unwrap_or(rate.min_amount))
        .bind(dto.max_amount.unwrap_or(rate.max_amount))
        .bind(rate.is_active)
        .bind(&rate.last_update_source)
        .fetch_one(&self.db)
        .await?;

        // Record in history
        self.record_rate_history(&saved_rate).await?;

        Ok(saved_rate)
    }

    async fn update(&self, id: Uuid, changeur_id: Uuid, dto: UpdateRateDto) -> Result<ExchangeRate, AppError> {
        // Verify ownership
        let existing = sqlx::query_as::<_, ExchangeRate>(
            "SELECT * FROM exchange_rates WHERE id = $1 AND changeur_id = $2"
        )
        .bind(id)
        .bind(changeur_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::not_found("Rate"))?;

        let mut query_parts = vec!["UPDATE exchange_rates SET updated_at = NOW()".to_string()];
        let mut bindings = vec![];
        let mut bind_count = 1;

        if let Some(buy_rate) = dto.buy_rate {
            query_parts.push(format!("buy_rate = ${}", bind_count));
            bindings.push(buy_rate.to_string());
            bind_count += 1;
        }

        if let Some(sell_rate) = dto.sell_rate {
            query_parts.push(format!("sell_rate = ${}", bind_count));
            bindings.push(sell_rate.to_string());
            bind_count += 1;
        }

        if dto.buy_rate.is_some() || dto.sell_rate.is_some() {
            // Recalculate mid_rate and spread
            let buy = dto.buy_rate.unwrap_or(existing.buy_rate);
            let sell = dto.sell_rate.unwrap_or(existing.sell_rate);
            let mid_rate = (buy + sell) / Decimal::from(2);
            let spread = sell - buy;
            
            query_parts.push(format!("mid_rate = ${}", bind_count));
            bindings.push(mid_rate.to_string());
            bind_count += 1;
            
            query_parts.push(format!("spread = ${}", bind_count));
            bindings.push(spread.to_string());
            bind_count += 1;
        }

        if let Some(available_amount) = dto.available_amount {
            query_parts.push(format!("available_amount = ${}", bind_count));
            bindings.push(available_amount.to_string());
            bind_count += 1;
        }

        if let Some(min_amount) = dto.min_amount {
            query_parts.push(format!("min_amount = ${}", bind_count));
            bindings.push(min_amount.to_string());
            bind_count += 1;
        }

        if let Some(max_amount) = dto.max_amount {
            query_parts.push(format!("max_amount = ${}", bind_count));
            bindings.push(max_amount.to_string());
            bind_count += 1;
        }

        if let Some(is_active) = dto.is_active {
            query_parts.push(format!("is_active = ${}", bind_count));
            bindings.push(is_active.to_string());
            bind_count += 1;
        }

        query_parts.push(format!("WHERE id = ${} AND changeur_id = ${}", bind_count, bind_count + 1));
        bindings.push(id.to_string());
        bindings.push(changeur_id.to_string());

        let query = query_parts.join(", ") + " RETURNING *";
        
        let mut query_builder = sqlx::query_as::<_, ExchangeRate>(&query);
        for binding in bindings {
            query_builder = query_builder.bind(binding);
        }

        let updated_rate = query_builder.fetch_one(&self.db).await?;

        // Record in history if rates changed
        if dto.buy_rate.is_some() || dto.sell_rate.is_some() {
            self.record_rate_history(&updated_rate).await?;
        }

        Ok(updated_rate)
    }

    async fn delete(&self, id: Uuid, changeur_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE exchange_rates SET is_active = false, updated_at = NOW() 
             WHERE id = $1 AND changeur_id = $2"
        )
        .bind(id)
        .bind(changeur_id)
        .execute(&self.db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("Rate"));
        }

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ExchangeRate>, AppError> {
        let rate = sqlx::query_as::<_, ExchangeRate>(
            "SELECT * FROM exchange_rates WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        Ok(rate)
    }

    async fn find_by_changeur(&self, changeur_id: Uuid) -> Result<Vec<ExchangeRate>, AppError> {
        let rates = sqlx::query_as::<_, ExchangeRate>(
            "SELECT * FROM exchange_rates 
             WHERE changeur_id = $1 
             ORDER BY from_currency, to_currency"
        )
        .bind(changeur_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rates)
    }

    async fn find_active_by_pair(&self, from: &str, to: &str) -> Result<Vec<ExchangeRate>, AppError> {
        let rates = sqlx::query_as::<_, ExchangeRate>(
            r#"
            SELECT r.* FROM exchange_rates r
            INNER JOIN users u ON r.changeur_id = u.id
            WHERE r.from_currency = $1 
            AND r.to_currency = $2 
            AND r.is_active = true
            AND u.is_active = true
            ORDER BY r.sell_rate ASC, r.buy_rate DESC
            "#
        )
        .bind(from.to_uppercase())
        .bind(to.to_uppercase())
        .fetch_all(&self.db)
        .await?;

        Ok(rates)
    }

    async fn get_public_rates(&self) -> Result<Vec<PublicRate>, AppError> {
        let rates = sqlx::query_as::<_, PublicRate>(
            r#"
            SELECT 
                from_currency,
                to_currency,
                MIN(buy_rate) as best_buy_rate,
                MIN(sell_rate) as best_sell_rate,
                AVG(buy_rate) as average_buy_rate,
                AVG(sell_rate) as average_sell_rate,
                AVG(spread) as spread,
                COUNT(DISTINCT changeur_id) as changeurs_count,
                MAX(updated_at) as last_updated
            FROM exchange_rates
            WHERE is_active = true
            GROUP BY from_currency, to_currency
            ORDER BY from_currency, to_currency
            "#
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rates)
    }

    async fn get_best_rates(
        &self, 
        from: &str, 
        to: &str, 
        amount: Decimal
    ) -> Result<Vec<ExchangeRate>, AppError> {
        let rates = sqlx::query_as::<_, ExchangeRate>(
            r#"
            SELECT r.* FROM exchange_rates r
            INNER JOIN users u ON r.changeur_id = u.id
            INNER JOIN changeur_profiles cp ON u.id = cp.user_id
            WHERE r.from_currency = $1 
            AND r.to_currency = $2 
            AND r.is_active = true
            AND u.is_active = true
            AND cp.is_verified = true
            AND r.min_amount <= $3
            AND r.max_amount >= $3
            AND (r.available_amount IS NULL OR r.available_amount >= $3)
            ORDER BY r.sell_rate ASC, cp.rating DESC
            LIMIT 5
            "#
        )
        .bind(from.to_uppercase())
        .bind(to.to_uppercase())
        .bind(amount)
        .fetch_all(&self.db)
        .await?;

        Ok(rates)
    }

    async fn record_rate_history(&self, rate: &ExchangeRate) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO exchange_rates_history (
                id, rate_id, changeur_id, from_currency, to_currency,
                buy_rate, sell_rate, mid_rate, recorded_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            "#
        )
        .bind(Uuid::new_v4())
        .bind(rate.id)
        .bind(rate.changeur_id)
        .bind(&rate.from_currency)
        .bind(&rate.to_currency)
        .bind(rate.buy_rate)
        .bind(rate.sell_rate)
        .bind(rate.mid_rate)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn detect_manipulation(&self, from: &str, to: &str, rate: Decimal) -> Result<bool, AppError> {
        use rust_decimal::prelude::FromPrimitive;
        
        // Get average rate for this pair
        let avg_rate: Option<Decimal> = sqlx::query_scalar(
            "SELECT AVG(buy_rate) FROM exchange_rates 
             WHERE from_currency = $1 AND to_currency = $2 AND is_active = true"
        )
        .bind(from)
        .bind(to)
        .fetch_optional(&self.db)
        .await?;
        
        if let Some(avg) = avg_rate {
            // Check if rate deviates more than 20% from average
            let deviation = ((rate - avg) / avg).abs();
            let threshold = Decimal::from_f64(0.2).unwrap_or(Decimal::from(0));
            Ok(deviation > threshold)
        } else {
            // No data to compare, consider it valid
            Ok(false)
        }
    }

    async fn get_rate_history(&self, from: &str, to: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<RateHistory>, AppError> {
        let history = sqlx::query_as::<_, RateHistory>(
            r#"
            SELECT changeur_id, buy_rate, sell_rate, mid_rate, recorded_at
            FROM exchange_rates_history
            WHERE from_currency = $1 
            AND to_currency = $2
            AND recorded_at BETWEEN $3 AND $4
            ORDER BY recorded_at DESC
            "#
        )
        .bind(from)
        .bind(to)
        .bind(start)
        .bind(end)
        .fetch_all(&self.db)
        .await?;
        
        Ok(history)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RateHistory {
    pub changeur_id: Uuid,
    pub buy_rate: Decimal,
    pub sell_rate: Decimal,
    pub mid_rate: Decimal,
    pub recorded_at: DateTime<Utc>,
}

// Repository for analytics
impl RateRepository {
    pub async fn get_rate_history(
        &self,
        from: &str,
        to: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<RateHistory>, AppError> {

        let history = sqlx::query_as::<_, RateHistory>(
            r#"
            SELECT changeur_id, buy_rate, sell_rate, mid_rate, recorded_at
            FROM exchange_rates_history
            WHERE from_currency = $1 
            AND to_currency = $2
            AND recorded_at BETWEEN $3 AND $4
            ORDER BY recorded_at DESC
            "#
        )
        .bind(from)
        .bind(to)
        .bind(start)
        .bind(end)
        .fetch_all(&self.db)
        .await?;

        Ok(history)
    }

    pub async fn detect_manipulation(
        &self,
        changeur_id: Uuid,
        from: &str,
        to: &str,
        proposed_rate: Decimal,
    ) -> Result<bool, AppError> {
        // Get average rate from other changeurs
        let avg_rate = sqlx::query(
            r#"
            SELECT AVG(sell_rate) as avg_rate
            FROM exchange_rates
            WHERE from_currency = $1 
            AND to_currency = $2
            AND changeur_id != $3
            AND is_active = true
            "#
        )
        .bind(from)
        .bind(to)
        .bind(changeur_id)
        .fetch_one(&self.db)
        .await?;

        let avg: Option<Decimal> = avg_rate.try_get("avg_rate")?;
        
        if let Some(average) = avg {
            // If rate deviates more than 10% from average, flag as potential manipulation
            let deviation = ((proposed_rate - average) / average * Decimal::from(100)).abs();
            return Ok(deviation > Decimal::from(10));
        }

        Ok(false)
    }
}