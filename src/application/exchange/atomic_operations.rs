use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Transaction as SqlxTransaction};
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::shared::errors::AppError;

/// Result of an atomic transfer operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicTransferResult {
    pub from_wallet_id: Uuid,
    pub to_wallet_id: Uuid,
    pub amount: Decimal,
    pub from_balance_after: Decimal,
    pub to_balance_after: Decimal,
    pub transaction_id: Uuid,
    pub completed_at: DateTime<Utc>,
}

/// Result of an atomic reserve operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicReserveResult {
    pub wallet_id: Uuid,
    pub amount_reserved: Decimal,
    pub available_after: Decimal,
    pub reserved_after: Decimal,
    pub reservation_id: Uuid,
}

/// Service for atomic wallet operations
pub struct AtomicWalletService {
    db: Pool<Postgres>,
}

impl AtomicWalletService {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }

    /// Atomic transfer between two wallets using a single SQL transaction
    /// This prevents any possibility of money loss or duplication
    pub async fn transfer_atomic(
        &self,
        from_wallet_id: Uuid,
        to_wallet_id: Uuid,
        amount: Decimal,
        currency: &str,
        transaction_id: Uuid,
        reference: &str,
    ) -> Result<AtomicTransferResult, AppError> {
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("Amount must be positive"));
        }

        if from_wallet_id == to_wallet_id {
            return Err(AppError::validation("Cannot transfer to same wallet"));
        }

        // Execute the entire transfer in a single atomic SQL query
        let result = sqlx::query!(
            r#"
            WITH validation AS (
                -- First validate both wallets exist and are active
                SELECT 
                    (SELECT COUNT(*) = 2 FROM wallets 
                     WHERE id IN ($1, $2) 
                     AND currency = $3
                     AND is_active = true) as wallets_valid,
                    (SELECT balance >= $4 FROM wallets 
                     WHERE id = $1) as has_sufficient_funds,
                    (SELECT balance - reserved_balance >= $4 FROM wallets 
                     WHERE id = $1) as has_available_funds
            ),
            transfer_check AS (
                -- Only proceed if validation passes
                SELECT 1 as proceed
                FROM validation
                WHERE wallets_valid = true 
                  AND has_sufficient_funds = true 
                  AND has_available_funds = true
            ),
            debit_operation AS (
                -- Debit from source wallet with FOR UPDATE lock
                UPDATE wallets 
                SET 
                    balance = balance - $4,
                    last_transaction_id = $5,
                    updated_at = NOW()
                WHERE id = $1
                  AND EXISTS (SELECT 1 FROM transfer_check)
                  AND balance >= $4
                  AND balance - reserved_balance >= $4
                RETURNING id, balance as from_balance
            ),
            credit_operation AS (
                -- Credit to destination wallet only if debit succeeded
                UPDATE wallets 
                SET 
                    balance = balance + $4,
                    last_transaction_id = $5,
                    updated_at = NOW()
                WHERE id = $2
                  AND EXISTS (SELECT 1 FROM debit_operation)
                RETURNING id, balance as to_balance
            ),
            transaction_record AS (
                -- Record the transaction only if both operations succeeded
                INSERT INTO wallet_transactions 
                    (id, from_wallet_id, to_wallet_id, amount, currency, 
                     transaction_id, reference, status, created_at)
                SELECT 
                    gen_random_uuid(), $1, $2, $4, $3, 
                    $5, $6, 'completed', NOW()
                WHERE EXISTS (SELECT 1 FROM debit_operation)
                  AND EXISTS (SELECT 1 FROM credit_operation)
                RETURNING id as record_id, created_at
            )
            SELECT 
                COALESCE((SELECT from_balance FROM debit_operation), -1) as from_balance,
                COALESCE((SELECT to_balance FROM credit_operation), -1) as to_balance,
                COALESCE((SELECT record_id FROM transaction_record), '00000000-0000-0000-0000-000000000000'::uuid) as record_id,
                COALESCE((SELECT created_at FROM transaction_record), NOW()) as completed_at,
                EXISTS (SELECT 1 FROM debit_operation) as debit_success,
                EXISTS (SELECT 1 FROM credit_operation) as credit_success,
                EXISTS (SELECT 1 FROM transaction_record) as record_success,
                (SELECT has_sufficient_funds FROM validation) as had_funds,
                (SELECT has_available_funds FROM validation) as had_available,
                (SELECT wallets_valid FROM validation) as wallets_were_valid
            "#,
            from_wallet_id,  // $1
            to_wallet_id,    // $2
            currency,        // $3
            amount,          // $4
            transaction_id,  // $5
            reference,       // $6
        )
        .fetch_one(&self.db)
        .await?;

        // Check if the transfer was successful
        let debit_success = result.debit_success.unwrap_or(false);
        let credit_success = result.credit_success.unwrap_or(false);
        let record_success = result.record_success.unwrap_or(false);
        
        if !debit_success || !credit_success || !record_success {
            if !result.wallets_were_valid.unwrap_or(false) {
                return Err(AppError::not_found("One or both wallets not found or inactive"));
            }
            if !result.had_funds.unwrap_or(false) {
                return Err(AppError::InsufficientFunds);
            }
            if !result.had_available.unwrap_or(false) {
                return Err(AppError::bad_request("Insufficient available balance (funds may be reserved)"));
            }
            return Err(AppError::InternalServerError);
        }

        info!(
            from_wallet_id = %from_wallet_id,
            to_wallet_id = %to_wallet_id,
            amount = %amount,
            currency = currency,
            reference = reference,
            "Atomic transfer completed successfully"
        );

        Ok(AtomicTransferResult {
            from_wallet_id,
            to_wallet_id,
            amount,
            from_balance_after: result.from_balance.unwrap_or(Decimal::ZERO),
            to_balance_after: result.to_balance.unwrap_or(Decimal::ZERO),
            transaction_id,
            completed_at: result.completed_at.unwrap_or_else(Utc::now),
        })
    }

    /// Atomically reserve funds in a wallet
    pub async fn reserve_funds_atomic(
        &self,
        wallet_id: Uuid,
        amount: Decimal,
        currency: &str,
        reference: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<AtomicReserveResult, AppError> {
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("Amount must be positive"));
        }

        let reservation_id = Uuid::new_v4();

        // Use a transaction with FOR UPDATE lock
        let mut tx = self.db.begin().await?;

        // Lock the wallet row for the duration of the transaction
        let wallet = sqlx::query!(
            r#"
            SELECT id, balance, reserved_balance, is_active
            FROM wallets 
            WHERE id = $1 AND currency = $2
            FOR UPDATE
            "#,
            wallet_id,
            currency
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found("Wallet not found"))?;

        if !wallet.is_active.unwrap_or(true) {
            tx.rollback().await?;
            return Err(AppError::bad_request("Wallet is not active"));
        }

        let available = wallet.balance - wallet.reserved_balance;
        if available < amount {
            tx.rollback().await?;
            warn!(
                wallet_id = %wallet_id,
                requested = %amount,
                available = %available,
                "Insufficient funds for reservation"
            );
            return Err(AppError::InsufficientFunds);
        }

        // Update the wallet with the reservation
        let updated = sqlx::query!(
            r#"
            UPDATE wallets 
            SET 
                reserved_balance = reserved_balance + $1,
                updated_at = NOW()
            WHERE id = $2
            RETURNING balance, reserved_balance
            "#,
            amount,
            wallet_id
        )
        .fetch_one(&mut *tx)
        .await?;

        // Record the reservation
        sqlx::query!(
            r#"
            INSERT INTO wallet_reservations 
                (id, wallet_id, amount, reference, expires_at, status, created_at)
            VALUES ($1, $2, $3, $4, $5, 'active', NOW())
            "#,
            reservation_id,
            wallet_id,
            amount,
            reference,
            expires_at
        )
        .execute(&mut *tx)
        .await?;

        // Commit the transaction
        tx.commit().await?;

        info!(
            wallet_id = %wallet_id,
            amount = %amount,
            reservation_id = %reservation_id,
            "Funds reserved atomically"
        );

        Ok(AtomicReserveResult {
            wallet_id,
            amount_reserved: amount,
            available_after: updated.balance - updated.reserved_balance,
            reserved_after: updated.reserved_balance,
            reservation_id,
        })
    }

    /// Atomically release reserved funds
    pub async fn release_funds_atomic(
        &self,
        wallet_id: Uuid,
        amount: Decimal,
        reservation_id: Option<Uuid>,
        reference: &str,
    ) -> Result<Decimal, AppError> {
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("Amount must be positive"));
        }

        let mut tx = self.db.begin().await?;

        // Lock the wallet and update in one operation
        let result = sqlx::query!(
            r#"
            UPDATE wallets 
            SET 
                reserved_balance = GREATEST(0, reserved_balance - $1),
                updated_at = NOW()
            WHERE id = $2 
              AND reserved_balance >= $1
            RETURNING balance, reserved_balance
            "#,
            amount,
            wallet_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let updated = match result {
            Some(w) => w,
            None => {
                tx.rollback().await?;
                warn!(
                    wallet_id = %wallet_id,
                    amount = %amount,
                    "Cannot release more than reserved amount"
                );
                return Err(AppError::bad_request("Cannot release more than reserved"));
            }
        };

        // Update reservation status if provided
        if let Some(res_id) = reservation_id {
            sqlx::query!(
                r#"
                UPDATE wallet_reservations 
                SET status = 'released', updated_at = NOW()
                WHERE id = $1
                "#,
                res_id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        info!(
            wallet_id = %wallet_id,
            amount = %amount,
            reference = reference,
            "Funds released atomically"
        );

        Ok(updated.balance - updated.reserved_balance)
    }

    /// Complete a reserved transfer atomically
    /// This converts reserved funds into an actual transfer
    pub async fn complete_reserved_transfer(
        &self,
        from_wallet_id: Uuid,
        to_wallet_id: Uuid,
        amount: Decimal,
        reserved_amount: Decimal,
        currency: &str,
        transaction_id: Uuid,
        reference: &str,
    ) -> Result<AtomicTransferResult, AppError> {
        // Execute everything in a single transaction
        let mut tx = self.db.begin().await?;

        // First, debit from source (converting reserved to transferred)
        let from_result = sqlx::query!(
            r#"
            UPDATE wallets 
            SET 
                balance = balance - $1,
                reserved_balance = GREATEST(0, reserved_balance - $2),
                last_transaction_id = $3,
                updated_at = NOW()
            WHERE id = $4
              AND balance >= $1
              AND reserved_balance >= $2
            RETURNING balance, reserved_balance
            "#,
            amount,
            reserved_amount,
            transaction_id,
            from_wallet_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let from_wallet = match from_result {
            Some(w) => w,
            None => {
                tx.rollback().await?;
                error!(
                    from_wallet_id = %from_wallet_id,
                    amount = %amount,
                    reserved = %reserved_amount,
                    "Failed to complete reserved transfer - insufficient funds or reservation"
                );
                return Err(AppError::InsufficientFunds);
            }
        };

        // Then credit to destination
        let to_result = sqlx::query!(
            r#"
            UPDATE wallets 
            SET 
                balance = balance + $1,
                last_transaction_id = $2,
                updated_at = NOW()
            WHERE id = $3
            RETURNING balance
            "#,
            amount,
            transaction_id,
            to_wallet_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let to_wallet = match to_result {
            Some(w) => w,
            None => {
                tx.rollback().await?;
                return Err(AppError::not_found("Destination wallet not found"));
            }
        };

        // Record the transaction
        sqlx::query!(
            r#"
            INSERT INTO wallet_transactions 
                (id, from_wallet_id, to_wallet_id, amount, currency, 
                 transaction_id, reference, status, created_at)
            VALUES 
                (gen_random_uuid(), $1, $2, $3, $4, $5, $6, 'completed', NOW())
            "#,
            from_wallet_id,
            to_wallet_id,
            amount,
            currency,
            transaction_id,
            reference
        )
        .execute(&mut *tx)
        .await?;

        // Commit everything
        tx.commit().await?;

        info!(
            from_wallet_id = %from_wallet_id,
            to_wallet_id = %to_wallet_id,
            amount = %amount,
            "Reserved transfer completed atomically"
        );

        Ok(AtomicTransferResult {
            from_wallet_id,
            to_wallet_id,
            amount,
            from_balance_after: from_wallet.balance,
            to_balance_after: to_wallet.balance,
            transaction_id,
            completed_at: Utc::now(),
        })
    }

    /// Check and expire old reservations atomically
    pub async fn expire_old_reservations(&self) -> Result<u32, AppError> {
        let result = sqlx::query!(
            r#"
            WITH expired_reservations AS (
                SELECT wallet_id, SUM(amount) as total_amount
                FROM wallet_reservations
                WHERE status = 'active' 
                  AND expires_at < NOW()
                GROUP BY wallet_id
            ),
            updated_wallets AS (
                UPDATE wallets w
                SET 
                    reserved_balance = GREATEST(0, reserved_balance - er.total_amount),
                    updated_at = NOW()
                FROM expired_reservations er
                WHERE w.id = er.wallet_id
                RETURNING w.id
            ),
            updated_reservations AS (
                UPDATE wallet_reservations
                SET 
                    status = 'expired',
                    updated_at = NOW()
                WHERE status = 'active' 
                  AND expires_at < NOW()
                RETURNING id
            )
            SELECT COUNT(*) as expired_count FROM updated_reservations
            "#
        )
        .fetch_one(&self.db)
        .await?;

        let count = result.expired_count.unwrap_or(0) as u32;
        
        if count > 0 {
            info!(count = count, "Expired reservations cleaned up");
        }

        Ok(count)
    }
}