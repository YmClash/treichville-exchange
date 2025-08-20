use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Transaction as SqlxTransaction};
use std::collections::HashMap;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::{
    domain::transaction::{Transaction, TransactionType},
    shared::errors::AppError,
};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Wallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub currency: String,
    pub balance: Decimal,
    pub reserved_balance: Decimal,
    pub last_transaction_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub currency: String,
    pub available_balance: Decimal,
    pub reserved_balance: Decimal,
    pub total_balance: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    pub wallet_id: Uuid,
    pub transaction_type: WalletTransactionType,
    pub amount: Decimal,
    pub balance_before: Decimal,
    pub balance_after: Decimal,
    pub reference: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalletTransactionType {
    Credit,
    Debit,
    Reserve,
    Release,
}

#[async_trait]
pub trait WalletServiceTrait: Send + Sync {
    async fn get_or_create_wallet(&self, user_id: Uuid, currency: &str) -> Result<Wallet, AppError>;
    async fn get_balance(&self, user_id: Uuid, currency: &str) -> Result<WalletBalance, AppError>;
    async fn get_all_balances(&self, user_id: Uuid) -> Result<Vec<WalletBalance>, AppError>;
    async fn credit(&self, user_id: Uuid, currency: &str, amount: Decimal, reference: &str) -> Result<Wallet, AppError>;
    async fn debit(&self, user_id: Uuid, currency: &str, amount: Decimal, reference: &str) -> Result<Wallet, AppError>;
    async fn reserve(&self, user_id: Uuid, currency: &str, amount: Decimal, reference: &str) -> Result<Wallet, AppError>;
    async fn release(&self, user_id: Uuid, currency: &str, amount: Decimal, reference: &str) -> Result<Wallet, AppError>;
    async fn transfer(&self, from_user_id: Uuid, to_user_id: Uuid, currency: &str, amount: Decimal, reference: &str) -> Result<(), AppError>;
}

pub struct WalletService {
    db: Pool<Postgres>,
}

impl WalletService {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }

    async fn get_wallet(&self, user_id: Uuid, currency: &str) -> Result<Option<Wallet>, AppError> {
        let wallet = sqlx::query_as::<_, Wallet>(
            "SELECT * FROM wallets WHERE user_id = $1 AND currency = $2"
        )
        .bind(user_id)
        .bind(currency.to_uppercase())
        .fetch_optional(&self.db)
        .await?;

        Ok(wallet)
    }

    async fn create_wallet(&self, user_id: Uuid, currency: &str) -> Result<Wallet, AppError> {
        let wallet = sqlx::query_as::<_, Wallet>(
            r#"
            INSERT INTO wallets (id, user_id, currency, balance, reserved_balance)
            VALUES ($1, $2, $3, 0, 0)
            RETURNING *
            "#
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(currency.to_uppercase())
        .fetch_one(&self.db)
        .await?;

        info!(
            user_id = %user_id,
            currency = currency,
            "New wallet created"
        );

        Ok(wallet)
    }

    async fn update_balance(
        &self,
        tx: &mut SqlxTransaction<'_, Postgres>,
        wallet_id: Uuid,
        balance_change: Decimal,
        reserved_change: Decimal,
        transaction_id: Option<Uuid>,
    ) -> Result<Wallet, AppError> {
        let wallet = sqlx::query_as::<_, Wallet>(
            r#"
            UPDATE wallets
            SET 
                balance = balance + $2,
                reserved_balance = reserved_balance + $3,
                last_transaction_id = COALESCE($4, last_transaction_id),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(wallet_id)
        .bind(balance_change)
        .bind(reserved_change)
        .bind(transaction_id)
        .fetch_one(&mut **tx)
        .await?;

        // Check for negative balance
        if wallet.balance < Decimal::ZERO {
            error!(
                wallet_id = %wallet_id,
                balance = %wallet.balance,
                "Negative balance detected!"
            );
            return Err(AppError::InsufficientFunds);
        }

        if wallet.reserved_balance < Decimal::ZERO {
            error!(
                wallet_id = %wallet_id,
                reserved = %wallet.reserved_balance,
                "Negative reserved balance detected!"
            );
            return Err(AppError::InternalServerError);
        }

        Ok(wallet)
    }

    async fn log_transaction(
        &self,
        tx: &mut SqlxTransaction<'_, Postgres>,
        wallet_id: Uuid,
        transaction_type: WalletTransactionType,
        amount: Decimal,
        balance_before: Decimal,
        balance_after: Decimal,
        reference: &str,
        description: Option<String>,
    ) -> Result<(), AppError> {
        // TODO: Create wallet_transactions table and implement logging
        info!(
            wallet_id = %wallet_id,
            transaction_type = ?transaction_type,
            amount = %amount,
            reference = reference,
            "Wallet transaction logged"
        );

        Ok(())
    }
}

#[async_trait]
impl WalletServiceTrait for WalletService {
    async fn get_or_create_wallet(&self, user_id: Uuid, currency: &str) -> Result<Wallet, AppError> {
        if let Some(wallet) = self.get_wallet(user_id, currency).await? {
            Ok(wallet)
        } else {
            self.create_wallet(user_id, currency).await
        }
    }

    async fn get_balance(&self, user_id: Uuid, currency: &str) -> Result<WalletBalance, AppError> {
        let wallet = self.get_or_create_wallet(user_id, currency).await?;

        Ok(WalletBalance {
            currency: wallet.currency,
            available_balance: wallet.balance - wallet.reserved_balance,
            reserved_balance: wallet.reserved_balance,
            total_balance: wallet.balance,
        })
    }

    async fn get_all_balances(&self, user_id: Uuid) -> Result<Vec<WalletBalance>, AppError> {
        let wallets = sqlx::query_as::<_, Wallet>(
            "SELECT * FROM wallets WHERE user_id = $1 ORDER BY currency"
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        let balances = wallets
            .into_iter()
            .map(|wallet| WalletBalance {
                currency: wallet.currency,
                available_balance: wallet.balance - wallet.reserved_balance,
                reserved_balance: wallet.reserved_balance,
                total_balance: wallet.balance,
            })
            .collect();

        Ok(balances)
    }

    async fn credit(
        &self,
        user_id: Uuid,
        currency: &str,
        amount: Decimal,
        reference: &str,
    ) -> Result<Wallet, AppError> {
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("Amount must be positive"));
        }

        let wallet = self.get_or_create_wallet(user_id, currency).await?;
        let balance_before = wallet.balance;

        let mut tx = self.db.begin().await?;

        let updated_wallet = self.update_balance(
            &mut tx,
            wallet.id,
            amount,
            Decimal::ZERO,
            None,
        ).await?;

        self.log_transaction(
            &mut tx,
            wallet.id,
            WalletTransactionType::Credit,
            amount,
            balance_before,
            updated_wallet.balance,
            reference,
            None,
        ).await?;

        tx.commit().await?;

        info!(
            user_id = %user_id,
            currency = currency,
            amount = %amount,
            reference = reference,
            "Wallet credited"
        );

        Ok(updated_wallet)
    }

    async fn debit(
        &self,
        user_id: Uuid,
        currency: &str,
        amount: Decimal,
        reference: &str,
    ) -> Result<Wallet, AppError> {
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("Amount must be positive"));
        }

        let wallet = self.get_wallet(user_id, currency).await?
            .ok_or_else(|| AppError::not_found("Wallet"))?;

        let available = wallet.balance - wallet.reserved_balance;
        if available < amount {
            warn!(
                user_id = %user_id,
                currency = currency,
                requested = %amount,
                available = %available,
                "Insufficient funds"
            );
            return Err(AppError::InsufficientFunds);
        }

        let balance_before = wallet.balance;

        let mut tx = self.db.begin().await?;

        let updated_wallet = self.update_balance(
            &mut tx,
            wallet.id,
            -amount,
            Decimal::ZERO,
            None,
        ).await?;

        self.log_transaction(
            &mut tx,
            wallet.id,
            WalletTransactionType::Debit,
            amount,
            balance_before,
            updated_wallet.balance,
            reference,
            None,
        ).await?;

        tx.commit().await?;

        info!(
            user_id = %user_id,
            currency = currency,
            amount = %amount,
            reference = reference,
            "Wallet debited"
        );

        Ok(updated_wallet)
    }

    async fn reserve(
        &self,
        user_id: Uuid,
        currency: &str,
        amount: Decimal,
        reference: &str,
    ) -> Result<Wallet, AppError> {
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("Amount must be positive"));
        }

        let wallet = self.get_wallet(user_id, currency).await?
            .ok_or_else(|| AppError::not_found("Wallet"))?;

        let available = wallet.balance - wallet.reserved_balance;
        if available < amount {
            warn!(
                user_id = %user_id,
                currency = currency,
                requested = %amount,
                available = %available,
                "Insufficient funds for reservation"
            );
            return Err(AppError::InsufficientFunds);
        }

        let mut tx = self.db.begin().await?;

        let updated_wallet = self.update_balance(
            &mut tx,
            wallet.id,
            Decimal::ZERO,
            amount,
            None,
        ).await?;

        self.log_transaction(
            &mut tx,
            wallet.id,
            WalletTransactionType::Reserve,
            amount,
            wallet.balance,
            wallet.balance, // Balance doesn't change, only reserved increases
            reference,
            Some(format!("Reserved {} {} for {}", amount, currency, reference)),
        ).await?;

        tx.commit().await?;

        info!(
            user_id = %user_id,
            currency = currency,
            amount = %amount,
            reference = reference,
            "Funds reserved"
        );

        Ok(updated_wallet)
    }

    async fn release(
        &self,
        user_id: Uuid,
        currency: &str,
        amount: Decimal,
        reference: &str,
    ) -> Result<Wallet, AppError> {
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("Amount must be positive"));
        }

        let wallet = self.get_wallet(user_id, currency).await?
            .ok_or_else(|| AppError::not_found("Wallet"))?;

        if wallet.reserved_balance < amount {
            warn!(
                user_id = %user_id,
                currency = currency,
                requested = %amount,
                reserved = %wallet.reserved_balance,
                "Cannot release more than reserved"
            );
            return Err(AppError::bad_request("Cannot release more than reserved amount"));
        }

        let mut tx = self.db.begin().await?;

        let updated_wallet = self.update_balance(
            &mut tx,
            wallet.id,
            Decimal::ZERO,
            -amount,
            None,
        ).await?;

        self.log_transaction(
            &mut tx,
            wallet.id,
            WalletTransactionType::Release,
            amount,
            wallet.balance,
            wallet.balance, // Balance doesn't change, only reserved decreases
            reference,
            Some(format!("Released {} {} for {}", amount, currency, reference)),
        ).await?;

        tx.commit().await?;

        info!(
            user_id = %user_id,
            currency = currency,
            amount = %amount,
            reference = reference,
            "Funds released"
        );

        Ok(updated_wallet)
    }

    async fn transfer(
        &self,
        from_user_id: Uuid,
        to_user_id: Uuid,
        currency: &str,
        amount: Decimal,
        reference: &str,
    ) -> Result<(), AppError> {
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("Amount must be positive"));
        }

        if from_user_id == to_user_id {
            return Err(AppError::validation("Cannot transfer to same user"));
        }

        // Start transaction
        let mut tx = self.db.begin().await?;

        // Get source wallet
        let from_wallet = self.get_wallet(from_user_id, currency).await?
            .ok_or_else(|| AppError::not_found("Source wallet"))?;

        let available = from_wallet.balance - from_wallet.reserved_balance;
        if available < amount {
            return Err(AppError::InsufficientFunds);
        }

        // Get or create destination wallet
        let to_wallet = self.get_or_create_wallet(to_user_id, currency).await?;

        // Debit source
        self.update_balance(
            &mut tx,
            from_wallet.id,
            -amount,
            Decimal::ZERO,
            None,
        ).await?;

        // Credit destination
        self.update_balance(
            &mut tx,
            to_wallet.id,
            amount,
            Decimal::ZERO,
            None,
        ).await?;

        // Log both transactions
        self.log_transaction(
            &mut tx,
            from_wallet.id,
            WalletTransactionType::Debit,
            amount,
            from_wallet.balance,
            from_wallet.balance - amount,
            reference,
            Some(format!("Transfer to user {}", to_user_id)),
        ).await?;

        self.log_transaction(
            &mut tx,
            to_wallet.id,
            WalletTransactionType::Credit,
            amount,
            to_wallet.balance,
            to_wallet.balance + amount,
            reference,
            Some(format!("Transfer from user {}", from_user_id)),
        ).await?;

        tx.commit().await?;

        info!(
            from_user_id = %from_user_id,
            to_user_id = %to_user_id,
            currency = currency,
            amount = %amount,
            reference = reference,
            "Transfer completed"
        );

        Ok(())
    }
}

// Additional public methods for WalletService
impl WalletService {
    pub async fn get_balance(&self, user_id: Uuid) -> Result<crate::domain::wallet::WalletBalance, AppError> {
        // Get default currency balance (XOF)
        self.get_balance_by_currency(user_id, "XOF").await
    }
    
    pub async fn get_balance_by_currency(&self, user_id: Uuid, currency: &str) -> Result<crate::domain::wallet::WalletBalance, AppError> {
        let wallet = self.get_wallet(user_id, currency).await?
            .ok_or_else(|| AppError::not_found("Wallet"))?;
        
        Ok(crate::domain::wallet::WalletBalance {
            total: wallet.balance,
            available: wallet.balance - wallet.reserved_balance,
            reserved: wallet.reserved_balance,
            currency: wallet.currency,
        })
    }
    
    pub async fn process_operation(&self, operation: crate::domain::wallet::WalletOperation) -> Result<crate::domain::wallet::WalletTransaction, AppError> {
        // This would process different wallet operations
        todo!("Implement process_operation")
    }
    
    pub async fn transfer(&self, request: crate::domain::wallet::TransferRequest) -> Result<crate::domain::wallet::WalletTransaction, AppError> {
        // Use the existing transfer method but return a transaction object
        let reference = format!("TRANSFER-{}", Uuid::new_v4());
        
        // TODO: Implement proper transaction return
        todo!("Implement transfer with TransferRequest")
    }
    
    pub async fn get_transactions(&self, user_id: Uuid, from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<crate::domain::wallet::WalletTransaction>, AppError> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);
        
        let mut query = String::from("SELECT * FROM wallet_transactions WHERE wallet_id IN (SELECT id FROM wallets WHERE user_id = $1)");
        
        if let Some(from_date) = from {
            query.push_str(&format!(" AND created_at >= '{}'", from_date));
        }
        
        if let Some(to_date) = to {
            query.push_str(&format!(" AND created_at <= '{}'", to_date));
        }
        
        query.push_str(&format!(" ORDER BY created_at DESC LIMIT {} OFFSET {}", limit, offset));
        
        let transactions = sqlx::query_as::<_, crate::domain::wallet::WalletTransaction>(&query)
            .bind(user_id)
            .fetch_all(&self.db)
            .await?;
        
        Ok(transactions)
    }
    
    pub async fn get_statistics(&self, user_id: Uuid, from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Result<crate::domain::wallet::WalletStatistics, AppError> {
        // Calculate wallet statistics
        let mut query = String::from(
            "SELECT 
                COALESCE(SUM(CASE WHEN amount > 0 THEN amount ELSE 0 END), 0) as total_in,
                COALESCE(SUM(CASE WHEN amount < 0 THEN ABS(amount) ELSE 0 END), 0) as total_out,
                COUNT(*) as transaction_count
            FROM wallet_transactions 
            WHERE wallet_id IN (SELECT id FROM wallets WHERE user_id = $1)"
        );
        
        if let Some(from_date) = from {
            query.push_str(&format!(" AND created_at >= '{}'", from_date));
        }
        
        if let Some(to_date) = to {
            query.push_str(&format!(" AND created_at <= '{}'", to_date));
        }
        
        #[derive(sqlx::FromRow)]
        struct StatsRow {
            total_in: Decimal,
            total_out: Decimal,
            transaction_count: i64,
        }
        
        let stats = sqlx::query_as::<_, StatsRow>(&query)
            .bind(user_id)
            .fetch_one(&self.db)
            .await?;
        
        Ok(crate::domain::wallet::WalletStatistics {
            total_in: stats.total_in,
            total_out: stats.total_out,
            transaction_count: stats.transaction_count,
        })
    }
    
    pub async fn reconcile_wallet(&self, user_id: Uuid, currency: &str) -> Result<crate::domain::wallet::ReconciliationResult, AppError> {
        // Reconcile wallet balance with transaction history
        let wallet = self.get_wallet(user_id, currency).await?
            .ok_or_else(|| AppError::not_found("Wallet"))?;
        
        // Calculate expected balance from transactions
        let sum_query = "
            SELECT COALESCE(SUM(amount), 0) as total
            FROM wallet_transactions
            WHERE wallet_id = $1
        ";
        
        let expected: Decimal = sqlx::query_scalar(sum_query)
            .bind(wallet.id)
            .fetch_one(&self.db)
            .await?;
        
        let actual = wallet.balance;
        let discrepancy = actual - expected;
        
        Ok(crate::domain::wallet::ReconciliationResult {
            expected_balance: expected,
            actual_balance: actual,
            discrepancy,
        })
    }
    
    pub async fn get_limits(&self, user_id: Uuid) -> Result<crate::domain::wallet::WalletLimits, AppError> {
        // TODO: Fetch from user settings or configuration
        Ok(crate::domain::wallet::WalletLimits {
            daily_limit: Decimal::from(1000000),
            monthly_limit: Decimal::from(10000000),
            single_transaction_limit: Decimal::from(500000),
        })
    }
    
    pub async fn get_pending_operations(&self, user_id: Uuid) -> Result<Vec<crate::domain::wallet::WalletOperation>, AppError> {
        // TODO: Implement fetching pending operations
        Ok(vec![])
    }
    
    pub async fn validate_operation(&self, user_id: Uuid, operation_type: &str, amount: Decimal, currency: &str) -> Result<crate::domain::wallet::ValidationResult, AppError> {
        let mut errors = Vec::new();
        
        if amount <= Decimal::ZERO {
            errors.push("Amount must be positive".to_string());
        }
        
        let wallet = self.get_wallet(user_id, currency).await?;
        
        if let Some(wallet) = wallet {
            let available = wallet.balance - wallet.reserved_balance;
            if operation_type == "withdrawal" && available < amount {
                errors.push("Insufficient funds".to_string());
            }
        } else {
            errors.push("Wallet not found".to_string());
        }
        
        Ok(crate::domain::wallet::ValidationResult {
            is_valid: errors.is_empty(),
            errors,
        })
    }
}