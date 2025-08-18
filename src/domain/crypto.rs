use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CryptoPrice {
    pub id: Uuid,
    pub symbol: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub price: Decimal,
    pub volume_24h: Option<Decimal>,
    pub change_24h: Option<Decimal>,
    pub source: String,
    pub recorded_at: DateTime<Utc>,
}

impl CryptoPrice {
    pub fn new(
        symbol: String,
        base_currency: String,
        quote_currency: String,
        price: Decimal,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            symbol: symbol.to_uppercase(),
            base_currency: base_currency.to_uppercase(),
            quote_currency: quote_currency.to_uppercase(),
            price,
            volume_24h: None,
            change_24h: None,
            source: "binance".to_string(),
            recorded_at: Utc::now(),
        }
    }

    pub fn convert_to_xof(&self, xof_rate: Decimal) -> Decimal {
        self.price * xof_rate
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoOrder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub order_type: CryptoOrderType,
    pub side: OrderSide,
    pub symbol: String,
    pub quantity: Decimal,
    pub price: Decimal,
    pub total_xof: Decimal,
    pub fee_xof: Decimal,
    pub status: OrderStatus,
    pub binance_order_id: Option<String>,
    pub executed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CryptoOrderType {
    Market,
    Limit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Pending,
    Executed,
    Failed,
    Cancelled,
}

impl CryptoOrder {
    pub fn new(
        user_id: Uuid,
        order_type: CryptoOrderType,
        side: OrderSide,
        symbol: String,
        quantity: Decimal,
        price: Decimal,
        xof_rate: Decimal,
        fee_percentage: Decimal,
    ) -> Self {
        let total_usd = quantity * price;
        let total_xof = total_usd * xof_rate;
        let fee_xof = crate::shared::utils::calculate_fee(total_xof, fee_percentage);

        Self {
            id: Uuid::new_v4(),
            user_id,
            order_type,
            side,
            symbol: symbol.to_uppercase(),
            quantity,
            price,
            total_xof,
            fee_xof,
            status: OrderStatus::Pending,
            binance_order_id: None,
            executed_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn mark_as_executed(&mut self, binance_order_id: String) {
        self.status = OrderStatus::Executed;
        self.binance_order_id = Some(binance_order_id);
        self.executed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_as_failed(&mut self) {
        self.status = OrderStatus::Failed;
        self.updated_at = Utc::now();
    }

    pub fn cancel(&mut self) {
        self.status = OrderStatus::Cancelled;
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoQuote {
    pub id: Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub quantity: Decimal,
    pub price_usd: Decimal,
    pub price_xof: Decimal,
    pub total_usd: Decimal,
    pub total_xof: Decimal,
    pub fee_xof: Decimal,
    pub grand_total_xof: Decimal,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl CryptoQuote {
    pub fn new(
        symbol: String,
        side: OrderSide,
        quantity: Decimal,
        price_usd: Decimal,
        xof_rate: Decimal,
        fee_percentage: Decimal,
    ) -> Self {
        let total_usd = quantity * price_usd;
        let price_xof = price_usd * xof_rate;
        let total_xof = total_usd * xof_rate;
        let fee_xof = crate::shared::utils::calculate_fee(total_xof, fee_percentage);
        let grand_total_xof = total_xof + fee_xof;

        Self {
            id: Uuid::new_v4(),
            symbol: symbol.to_uppercase(),
            side,
            quantity,
            price_usd,
            price_xof,
            total_usd,
            total_xof,
            fee_xof,
            grand_total_xof,
            expires_at: Utc::now() + chrono::Duration::minutes(5),
            created_at: Utc::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }
}