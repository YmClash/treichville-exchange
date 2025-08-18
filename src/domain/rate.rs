use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExchangeRate {
    pub id: Uuid,
    pub changeur_id: Uuid,
    pub from_currency: String,
    pub to_currency: String,
    pub buy_rate: Decimal,
    pub sell_rate: Decimal,
    pub mid_rate: Decimal,
    pub spread: Decimal,
    pub available_amount: Option<Decimal>,
    pub min_amount: Decimal,
    pub max_amount: Decimal,
    pub is_active: bool,
    pub last_update_source: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ExchangeRate {
    pub fn new(
        changeur_id: Uuid,
        from_currency: String,
        to_currency: String,
        buy_rate: Decimal,
        sell_rate: Decimal,
    ) -> Self {
        let mid_rate = (buy_rate + sell_rate) / Decimal::from(2);
        let spread = sell_rate - buy_rate;

        Self {
            id: Uuid::new_v4(),
            changeur_id,
            from_currency: from_currency.to_uppercase(),
            to_currency: to_currency.to_uppercase(),
            buy_rate,
            sell_rate,
            mid_rate,
            spread,
            available_amount: None,
            min_amount: Decimal::from(1000),
            max_amount: Decimal::from(10000000),
            is_active: true,
            last_update_source: Some("manual".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn calculate_exchange(&self, amount: Decimal, is_buying: bool) -> Decimal {
        if is_buying {
            amount * self.buy_rate
        } else {
            amount / self.sell_rate
        }
    }

    pub fn is_valid_amount(&self, amount: Decimal) -> bool {
        amount >= self.min_amount && amount <= self.max_amount
    }

    pub fn has_sufficient_liquidity(&self, amount: Decimal) -> bool {
        self.available_amount
            .map(|available| available >= amount)
            .unwrap_or(true)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateQuote {
    pub id: Uuid,
    pub from_currency: String,
    pub to_currency: String,
    pub amount_from: Decimal,
    pub amount_to: Decimal,
    pub rate: Decimal,
    pub fee: Decimal,
    pub total: Decimal,
    pub changeur_id: Uuid,
    pub changeur_name: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl RateQuote {
    pub fn new(
        rate: &ExchangeRate,
        amount: Decimal,
        is_buying: bool,
        fee_percentage: Decimal,
        changeur_name: String,
    ) -> Self {
        let amount_to = rate.calculate_exchange(amount, is_buying);
        let fee = crate::shared::utils::calculate_fee(amount, fee_percentage);
        let total = amount + fee;
        
        Self {
            id: Uuid::new_v4(),
            from_currency: rate.from_currency.clone(),
            to_currency: rate.to_currency.clone(),
            amount_from: amount,
            amount_to,
            rate: if is_buying { rate.buy_rate } else { rate.sell_rate },
            fee,
            total,
            changeur_id: rate.changeur_id,
            changeur_name,
            expires_at: Utc::now() + chrono::Duration::minutes(15),
            created_at: Utc::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateRateDto {
    #[validate(custom = "crate::shared::utils::validate_currency_code")]
    pub from_currency: String,
    
    #[validate(custom = "crate::shared::utils::validate_currency_code")]
    pub to_currency: String,
    
    #[validate(range(min = 0.0001))]
    pub buy_rate: Decimal,
    
    #[validate(range(min = 0.0001))]
    pub sell_rate: Decimal,
    
    pub available_amount: Option<Decimal>,
    
    #[validate(range(min = 0))]
    pub min_amount: Option<Decimal>,
    
    #[validate(range(min = 0))]
    pub max_amount: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateRateDto {
    #[validate(range(min = 0.0001))]
    pub buy_rate: Option<Decimal>,
    
    #[validate(range(min = 0.0001))]
    pub sell_rate: Option<Decimal>,
    
    pub available_amount: Option<Decimal>,
    
    #[validate(range(min = 0))]
    pub min_amount: Option<Decimal>,
    
    #[validate(range(min = 0))]
    pub max_amount: Option<Decimal>,
    
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GetQuoteRequest {
    #[validate(custom = "crate::shared::utils::validate_currency_code")]
    pub from_currency: String,
    
    #[validate(custom = "crate::shared::utils::validate_currency_code")]
    pub to_currency: String,
    
    #[validate(range(min = 0))]
    pub amount: Decimal,
    
    pub operation: ExchangeOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExchangeOperation {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicRate {
    pub from_currency: String,
    pub to_currency: String,
    pub best_buy_rate: Decimal,
    pub best_sell_rate: Decimal,
    pub average_buy_rate: Decimal,
    pub average_sell_rate: Decimal,
    pub spread: Decimal,
    pub changeurs_count: i64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestRateResponse {
    pub quotes: Vec<RateQuote>,
    pub best_quote: RateQuote,
    pub savings_vs_average: Decimal,
}