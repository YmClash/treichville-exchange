use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;

use crate::{
    domain::transaction::{
        Transaction, TransactionStatus, TransactionType, 
        PaymentMethod, TransactionEvent
    },
    shared::{errors::AppError, utils},
};

#[async_trait]
pub trait TransactionRepositoryTrait: Send + Sync {
    async fn create(&self, transaction: Transaction) -> Result<Transaction, AppError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Transaction>, AppError>;
    async fn find_by_reference(&self, reference: &str) -> Result<Option<Transaction>, AppError>;
    async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<Transaction>, AppError>;
    async fn find_by_user(&self, user_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Transaction>, AppError>;
    async fn update_status(&self, id: Uuid, status: TransactionStatus) -> Result<Transaction, AppError>;
    async fn update_payment_proof(&self, id: Uuid, payment_reference: String, proof: serde_json::Value) -> Result<Transaction, AppError>;
    async fn get_user_volume(&self, user_id: Uuid, period: DatePeriod) -> Result<TransactionVolume, AppError>;
    async fn get_expired_transactions(&self) -> Result<Vec<Transaction>, AppError>;
    async fn create_event(&self, event: TransactionEvent) -> Result<(), AppError>;
    async fn get_transaction_events(&self, transaction_id: Uuid) -> Result<Vec<TransactionEvent>, AppError>;
}

#[derive(Debug, Clone)]
pub enum DatePeriod {
    Today,
    ThisWeek,
    ThisMonth,
    Custom(DateTime<Utc>, DateTime<Utc>),
}

#[derive(Debug, Clone)]
pub struct TransactionVolume {
    pub total_count: i64,
    pub total_amount: Decimal,
    pub completed_count: i64,
    pub completed_amount: Decimal,
    pub pending_count: i64,
    pub pending_amount: Decimal,
}

pub struct TransactionRepository {
    db: Pool<Postgres>,
}

impl TransactionRepository {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }

    fn get_date_range(&self, period: &DatePeriod) -> (DateTime<Utc>, DateTime<Utc>) {
        match period {
            DatePeriod::Today => {
                let today = Utc::now().date_naive();
                let start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let end = today.and_hms_opt(23, 59, 59).unwrap().and_utc();
                (start, end)
            }
            DatePeriod::ThisWeek => {
                let now = Utc::now();
                let days_from_monday = now.weekday().num_days_from_monday();
                let monday = now - Duration::days(days_from_monday as i64);
                let sunday = monday + Duration::days(6);
                (
                    monday.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc(),
                    sunday.date_naive().and_hms_opt(23, 59, 59).unwrap().and_utc(),
                )
            }
            DatePeriod::ThisMonth => {
                let now = Utc::now();
                let start = now
                    .date_naive()
                    .with_day(1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();
                let next_month = if now.month() == 12 {
                    now.with_year(now.year() + 1)
                        .unwrap()
                        .with_month(1)
                        .unwrap()
                } else {
                    now.with_month(now.month() + 1).unwrap()
                };
                let end = next_month
                    .date_naive()
                    .with_day(1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    - Duration::seconds(1);
                (start, end)
            }
            DatePeriod::Custom(start, end) => (*start, *end),
        }
    }
}

#[async_trait]
impl TransactionRepositoryTrait for TransactionRepository {
    async fn create(&self, transaction: Transaction) -> Result<Transaction, AppError> {
        let saved = sqlx::query_as::<_, Transaction>(
            r#"
            INSERT INTO transactions (
                id, reference, client_id, changeur_id, type, status,
                from_currency, to_currency, amount_from, amount_to,
                rate_applied, fee, total_amount, payment_method,
                payment_reference, payment_proof, blockchain_tx_hash,
                notes, idempotency_key, expires_at, metadata
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21
            )
            RETURNING *
            "#
        )
        .bind(transaction.id)
        .bind(&transaction.reference)
        .bind(transaction.client_id)
        .bind(transaction.changeur_id)
        .bind(&transaction.transaction_type)
        .bind(&transaction.status)
        .bind(&transaction.from_currency)
        .bind(&transaction.to_currency)
        .bind(transaction.amount_from)
        .bind(transaction.amount_to)
        .bind(transaction.rate_applied)
        .bind(transaction.fee)
        .bind(transaction.total_amount)
        .bind(&transaction.payment_method)
        .bind(&transaction.payment_reference)
        .bind(&transaction.payment_proof)
        .bind(&transaction.blockchain_tx_hash)
        .bind(&transaction.notes)
        .bind(&transaction.idempotency_key)
        .bind(transaction.expires_at)
        .bind(&transaction.metadata)
        .fetch_one(&self.db)
        .await?;

        // Create initial event
        let event = TransactionEvent::new(
            saved.id,
            "created".to_string(),
            None,
            Some(saved.status.clone()),
            Some(saved.client_id),
            Some(format!("Transaction {} created", saved.reference)),
        );
        self.create_event(event).await?;

        Ok(saved)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Transaction>, AppError> {
        let transaction = sqlx::query_as::<_, Transaction>(
            "SELECT * FROM transactions WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        Ok(transaction)
    }

    async fn find_by_reference(&self, reference: &str) -> Result<Option<Transaction>, AppError> {
        let transaction = sqlx::query_as::<_, Transaction>(
            "SELECT * FROM transactions WHERE reference = $1"
        )
        .bind(reference)
        .fetch_optional(&self.db)
        .await?;

        Ok(transaction)
    }

    async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<Transaction>, AppError> {
        let transaction = sqlx::query_as::<_, Transaction>(
            "SELECT * FROM transactions WHERE idempotency_key = $1"
        )
        .bind(key)
        .fetch_optional(&self.db)
        .await?;

        Ok(transaction)
    }

    async fn find_by_user(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Transaction>, AppError> {
        let transactions = sqlx::query_as::<_, Transaction>(
            r#"
            SELECT * FROM transactions 
            WHERE client_id = $1 OR changeur_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        Ok(transactions)
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: TransactionStatus,
    ) -> Result<Transaction, AppError> {
        // Get current transaction for event logging
        let current = self.find_by_id(id).await?
            .ok_or_else(|| AppError::not_found("Transaction"))?;

        let timestamp_field = match status {
            TransactionStatus::Paid => "paid_at",
            TransactionStatus::Confirmed => "confirmed_at",
            TransactionStatus::Completed => "completed_at",
            TransactionStatus::Cancelled | TransactionStatus::Failed => "cancelled_at",
            _ => "updated_at",
        };

        let query = format!(
            r#"
            UPDATE transactions 
            SET status = $1, {} = NOW(), updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#,
            timestamp_field
        );

        let updated = sqlx::query_as::<_, Transaction>(&query)
            .bind(&status)
            .bind(id)
            .fetch_one(&self.db)
            .await?;

        // Create status change event
        let event = TransactionEvent::new(
            id,
            "status_changed".to_string(),
            Some(current.status),
            Some(status),
            None,
            Some(format!("Status changed from {:?} to {:?}", current.status, status)),
        );
        self.create_event(event).await?;

        Ok(updated)
    }

    async fn update_payment_proof(
        &self,
        id: Uuid,
        payment_reference: String,
        proof: serde_json::Value,
    ) -> Result<Transaction, AppError> {
        let updated = sqlx::query_as::<_, Transaction>(
            r#"
            UPDATE transactions 
            SET payment_reference = $1, payment_proof = $2, 
                status = 'paid', paid_at = NOW(), updated_at = NOW()
            WHERE id = $3
            RETURNING *
            "#
        )
        .bind(&payment_reference)
        .bind(&proof)
        .bind(id)
        .fetch_one(&self.db)
        .await?;

        // Create payment event
        let event = TransactionEvent::new(
            id,
            "payment_received".to_string(),
            Some(TransactionStatus::Pending),
            Some(TransactionStatus::Paid),
            None,
            Some(format!("Payment received with reference: {}", payment_reference)),
        );
        self.create_event(event).await?;

        Ok(updated)
    }

    async fn get_user_volume(
        &self,
        user_id: Uuid,
        period: DatePeriod,
    ) -> Result<TransactionVolume, AppError> {
        let (start, end) = self.get_date_range(&period);

        let result = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_count,
                COALESCE(SUM(total_amount), 0) as total_amount,
                COUNT(*) FILTER (WHERE status = 'completed') as completed_count,
                COALESCE(SUM(total_amount) FILTER (WHERE status = 'completed'), 0) as completed_amount,
                COUNT(*) FILTER (WHERE status IN ('pending', 'paid', 'confirmed')) as pending_count,
                COALESCE(SUM(total_amount) FILTER (WHERE status IN ('pending', 'paid', 'confirmed')), 0) as pending_amount
            FROM transactions
            WHERE (client_id = $1 OR changeur_id = $1)
            AND created_at BETWEEN $2 AND $3
            "#
        )
        .bind(user_id)
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await?;

        Ok(TransactionVolume {
            total_count: result.get("total_count"),
            total_amount: result.get("total_amount"),
            completed_count: result.get("completed_count"),
            completed_amount: result.get("completed_amount"),
            pending_count: result.get("pending_count"),
            pending_amount: result.get("pending_amount"),
        })
    }

    async fn get_expired_transactions(&self) -> Result<Vec<Transaction>, AppError> {
        let transactions = sqlx::query_as::<_, Transaction>(
            r#"
            SELECT * FROM transactions 
            WHERE status = 'pending' 
            AND expires_at IS NOT NULL 
            AND expires_at < NOW()
            "#
        )
        .fetch_all(&self.db)
        .await?;

        Ok(transactions)
    }

    async fn create_event(&self, event: TransactionEvent) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO transaction_events (
                id, transaction_id, event_type, previous_status, 
                new_status, actor_id, description, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(event.id)
        .bind(event.transaction_id)
        .bind(&event.event_type)
        .bind(&event.previous_status)
        .bind(&event.new_status)
        .bind(event.actor_id)
        .bind(&event.description)
        .bind(&event.metadata)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn get_transaction_events(
        &self,
        transaction_id: Uuid,
    ) -> Result<Vec<TransactionEvent>, AppError> {
        let events = sqlx::query_as::<_, TransactionEvent>(
            r#"
            SELECT * FROM transaction_events 
            WHERE transaction_id = $1 
            ORDER BY created_at ASC
            "#
        )
        .bind(transaction_id)
        .fetch_all(&self.db)
        .await?;

        Ok(events)
    }
}

// Additional analytics methods
impl TransactionRepository {
    pub async fn get_statistics(
        &self,
        changeur_id: Option<Uuid>,
        period: DatePeriod,
    ) -> Result<TransactionStatistics, AppError> {
        let (start, end) = self.get_date_range(&period);

        let mut query = r#"
            SELECT 
                COUNT(*) as total_transactions,
                COUNT(DISTINCT client_id) as unique_clients,
                COALESCE(SUM(total_amount), 0) as total_volume,
                COALESCE(AVG(total_amount), 0) as average_transaction,
                COUNT(*) FILTER (WHERE status = 'completed') as successful_transactions,
                COUNT(*) FILTER (WHERE status IN ('cancelled', 'failed', 'expired')) as failed_transactions,
                COALESCE(AVG(EXTRACT(EPOCH FROM (completed_at - created_at))) FILTER (WHERE status = 'completed'), 0) as avg_completion_time_seconds
            FROM transactions
            WHERE created_at BETWEEN $1 AND $2
        "#.to_string();

        if changeur_id.is_some() {
            query.push_str(" AND changeur_id = $3");
        }

        let mut query_builder = sqlx::query(&query)
            .bind(start)
            .bind(end);

        if let Some(id) = changeur_id {
            query_builder = query_builder.bind(id);
        }

        let result = query_builder.fetch_one(&self.db).await?;

        let total_transactions: i64 = result.get("total_transactions");
        let successful_transactions: i64 = result.get("successful_transactions");
        let success_rate = if total_transactions > 0 {
            (successful_transactions as f64 / total_transactions as f64) * 100.0
        } else {
            0.0
        };

        Ok(TransactionStatistics {
            total_transactions: result.get("total_transactions"),
            unique_clients: result.get("unique_clients"),
            total_volume: result.get("total_volume"),
            average_transaction: result.get("average_transaction"),
            successful_transactions,
            failed_transactions: result.get("failed_transactions"),
            success_rate: Decimal::from_f64_retain(success_rate).unwrap_or(Decimal::ZERO),
            avg_completion_time_seconds: result.get::<f64, _>("avg_completion_time_seconds") as i64,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TransactionStatistics {
    pub total_transactions: i64,
    pub unique_clients: i64,
    pub total_volume: Decimal,
    pub average_transaction: Decimal,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub success_rate: Decimal,
    pub avg_completion_time_seconds: i64,
}