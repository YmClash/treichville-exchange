use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transaction_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Exchange,
    CryptoBuy,
    CryptoSell,
    Deposit,
    Withdrawal,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transaction_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Pending,
    Paid,
    Confirmed,
    Completed,
    Cancelled,
    Expired,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "payment_method", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    Cash,
    OrangeMoney,
    Wave,
    MtnMoney,
    MoovMoney,
    BankTransfer,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub reference: String,
    pub client_id: Uuid,
    pub changeur_id: Option<Uuid>,
    #[sqlx(rename = "type")]
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub from_currency: String,
    pub to_currency: String,
    pub amount_from: Decimal,
    pub amount_to: Decimal,
    pub rate_applied: Decimal,
    pub fee: Decimal,
    pub total_amount: Decimal,
    pub payment_method: PaymentMethod,
    pub payment_reference: Option<String>,
    pub payment_proof: Option<serde_json::Value>,
    pub blockchain_tx_hash: Option<String>,
    pub notes: Option<String>,
    pub idempotency_key: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancelled_reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Transaction {
    pub fn new(
        client_id: Uuid,
        changeur_id: Option<Uuid>,
        transaction_type: TransactionType,
        from_currency: String,
        to_currency: String,
        amount_from: Decimal,
        amount_to: Decimal,
        rate_applied: Decimal,
        fee: Decimal,
        payment_method: PaymentMethod,
    ) -> Self {
        let reference = crate::shared::utils::generate_transaction_reference();
        let total_amount = amount_from + fee;
        let expires_at = Some(Utc::now() + chrono::Duration::minutes(15));

        Self {
            id: Uuid::new_v4(),
            reference,
            client_id,
            changeur_id,
            transaction_type,
            status: TransactionStatus::Pending,
            from_currency: from_currency.to_uppercase(),
            to_currency: to_currency.to_uppercase(),
            amount_from,
            amount_to,
            rate_applied,
            fee,
            total_amount,
            payment_method,
            payment_reference: None,
            payment_proof: None,
            blockchain_tx_hash: None,
            notes: None,
            idempotency_key: None,
            expires_at,
            paid_at: None,
            confirmed_at: None,
            completed_at: None,
            cancelled_at: None,
            cancelled_reason: None,
            metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now() && self.status == TransactionStatus::Pending
        } else {
            false
        }
    }

    pub fn can_be_cancelled(&self) -> bool {
        matches!(
            self.status,
            TransactionStatus::Pending | TransactionStatus::Paid
        )
    }

    pub fn can_be_confirmed(&self) -> bool {
        self.status == TransactionStatus::Paid
    }

    pub fn mark_as_paid(&mut self, payment_reference: String, payment_proof: Option<serde_json::Value>) {
        self.status = TransactionStatus::Paid;
        self.payment_reference = Some(payment_reference);
        self.payment_proof = payment_proof;
        self.paid_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_as_confirmed(&mut self) {
        self.status = TransactionStatus::Confirmed;
        self.confirmed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_as_completed(&mut self) {
        self.status = TransactionStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn cancel(&mut self, reason: String) {
        self.status = TransactionStatus::Cancelled;
        self.cancelled_at = Some(Utc::now());
        self.cancelled_reason = Some(reason);
        self.updated_at = Utc::now();
    }

    pub fn mark_as_expired(&mut self) {
        self.status = TransactionStatus::Expired;
        self.updated_at = Utc::now();
    }

    pub fn mark_as_failed(&mut self, reason: String) {
        self.status = TransactionStatus::Failed;
        self.cancelled_reason = Some(reason);
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEvent {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub event_type: String,
    pub previous_status: Option<TransactionStatus>,
    pub new_status: Option<TransactionStatus>,
    pub actor_id: Option<Uuid>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl TransactionEvent {
    pub fn new(
        transaction_id: Uuid,
        event_type: String,
        previous_status: Option<TransactionStatus>,
        new_status: Option<TransactionStatus>,
        actor_id: Option<Uuid>,
        description: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            transaction_id,
            event_type,
            previous_status,
            new_status,
            actor_id,
            description,
            metadata: None,
            created_at: Utc::now(),
        }
    }
}