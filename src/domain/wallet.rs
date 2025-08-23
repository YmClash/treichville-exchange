use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "wallet_operation", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WalletOperation {
    Deposit,
    Withdrawal,
    Transfer,
    Reserve,
    Release,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Wallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub currency: String,
    pub balance: Decimal,
    pub reserved_balance: Decimal,
    pub available_balance: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    pub from_wallet_id: Uuid,
    pub to_wallet_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WalletTransaction {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub operation: WalletOperation,
    pub amount: Decimal,
    pub balance_before: Decimal,
    pub balance_after: Decimal,
    pub reference: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Wallet {
    pub fn new(user_id: Uuid, currency: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            currency: currency.to_uppercase(),
            balance: Decimal::ZERO,
            reserved_balance: Decimal::ZERO,
            available_balance: Decimal::ZERO,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn can_withdraw(&self, amount: Decimal) -> bool {
        self.available_balance >= amount
    }

    pub fn reserve(&mut self, amount: Decimal) -> Result<(), String> {
        if self.available_balance < amount {
            return Err("Insufficient available balance".to_string());
        }
        self.reserved_balance += amount;
        self.available_balance -= amount;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn release(&mut self, amount: Decimal) -> Result<(), String> {
        if self.reserved_balance < amount {
            return Err("Amount exceeds reserved balance".to_string());
        }
        self.reserved_balance -= amount;
        self.available_balance += amount;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn deposit(&mut self, amount: Decimal) {
        self.balance += amount;
        self.available_balance += amount;
        self.updated_at = Utc::now();
    }

    pub fn withdraw(&mut self, amount: Decimal) -> Result<(), String> {
        if self.available_balance < amount {
            return Err("Insufficient available balance".to_string());
        }
        self.balance -= amount;
        self.available_balance -= amount;
        self.updated_at = Utc::now();
        Ok(())
    }
}

// Additional structs for wallet service methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub total: Decimal,
    pub available: Decimal,
    pub reserved: Decimal,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletStatistics {
    pub total_in: Decimal,
    pub total_out: Decimal,
    pub transaction_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationResult {
    pub expected_balance: Decimal,
    pub actual_balance: Decimal,
    pub discrepancy: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletLimits {
    pub daily_limit: Decimal,
    pub monthly_limit: Decimal,
    pub single_transaction_limit: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
}