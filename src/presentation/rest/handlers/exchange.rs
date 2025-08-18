use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
    Extension,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    application::{
        auth::middleware::CurrentUser,
        exchange::{
            TransactionService, CreateTransactionDto, ConfirmPaymentDto,
            TransactionResponse,
        },
    },
    shared::{
        errors::{AppError, AppResult},
        utils::PaginationParams,
    },
};

// POST /api/v1/exchange/initiate
pub async fn initiate_transaction(
    Extension(user): Extension<CurrentUser>,
    State(service): State<std::sync::Arc<TransactionService>>,
    Json(dto): Json<CreateTransactionDto>,
) -> AppResult<impl IntoResponse> {
    let transaction = service.create_transaction(user.id, dto).await?;
    
    Ok((StatusCode::CREATED, Json(transaction)))
}

// POST /api/v1/exchange/confirm
pub async fn confirm_payment(
    Extension(user): Extension<CurrentUser>,
    State(service): State<std::sync::Arc<TransactionService>>,
    Json(dto): Json<ConfirmPaymentDto>,
) -> AppResult<impl IntoResponse> {
    let transaction = service.confirm_payment(user.id, dto).await?;
    
    Ok((StatusCode::OK, Json(transaction)))
}

// POST /api/v1/exchange/:id/complete
pub async fn complete_transaction(
    Path(id): Path<Uuid>,
    Extension(user): Extension<CurrentUser>,
    State(service): State<std::sync::Arc<TransactionService>>,
) -> AppResult<impl IntoResponse> {
    // Only changeurs can complete transactions
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can complete transactions"));
    }

    let transaction = service.complete_transaction(id, user.id).await?;
    
    Ok((StatusCode::OK, Json(transaction)))
}

// POST /api/v1/exchange/:id/cancel
pub async fn cancel_transaction(
    Path(id): Path<Uuid>,
    Extension(user): Extension<CurrentUser>,
    State(service): State<std::sync::Arc<TransactionService>>,
    Json(body): Json<CancelRequest>,
) -> AppResult<impl IntoResponse> {
    let transaction = service.cancel_transaction(id, user.id, body.reason).await?;
    
    Ok((StatusCode::OK, Json(transaction)))
}

#[derive(Debug, Deserialize)]
pub struct CancelRequest {
    pub reason: String,
}

// GET /api/v1/exchange/:id
pub async fn get_transaction(
    Path(id): Path<Uuid>,
    Extension(user): Extension<CurrentUser>,
    State(service): State<std::sync::Arc<TransactionService>>,
) -> AppResult<impl IntoResponse> {
    let transaction = service.get_transaction(id, user.id).await?;
    
    Ok((StatusCode::OK, Json(transaction)))
}

// GET /api/v1/exchange
pub async fn get_user_transactions(
    Query(params): Query<PaginationParams>,
    Extension(user): Extension<CurrentUser>,
    State(service): State<std::sync::Arc<TransactionService>>,
) -> AppResult<impl IntoResponse> {
    let transactions = service
        .get_user_transactions(user.id, params.page, params.per_page)
        .await?;
    
    Ok((StatusCode::OK, Json(transactions)))
}

// GET /api/v1/exchange/statistics
pub async fn get_transaction_statistics(
    Query(params): Query<StatisticsQuery>,
    Extension(user): Extension<CurrentUser>,
    State(repository): State<std::sync::Arc<crate::application::exchange::TransactionRepository>>,
) -> AppResult<impl IntoResponse> {
    use crate::application::exchange::transaction_repository::DatePeriod;

    let period = match params.period.as_deref() {
        Some("today") => DatePeriod::Today,
        Some("week") => DatePeriod::ThisWeek,
        Some("month") => DatePeriod::ThisMonth,
        _ => DatePeriod::ThisMonth,
    };

    // For changeurs, get their specific stats
    let changeur_id = if user.is_changeur() {
        Some(user.id)
    } else {
        None
    };

    let stats = repository.get_statistics(changeur_id, period).await?;
    
    Ok((StatusCode::OK, Json(stats)))
}

#[derive(Debug, Deserialize)]
pub struct StatisticsQuery {
    pub period: Option<String>, // "today", "week", "month"
}

// POST /api/v1/exchange/webhook/:provider
pub async fn payment_webhook(
    Path(provider): Path<String>,
    State(processor): State<std::sync::Arc<crate::application::exchange::PaymentProcessor>>,
    Json(webhook): Json<serde_json::Value>,
) -> AppResult<impl IntoResponse> {
    use crate::application::exchange::payment_processor::{PaymentWebhook, PaymentProvider};

    // Parse provider
    let payment_provider = match provider.to_lowercase().as_str() {
        "orange_money" => PaymentProvider::OrangeMoney,
        "wave" => PaymentProvider::Wave,
        "mtn_money" => PaymentProvider::MtnMoney,
        _ => return Err(AppError::bad_request("Unknown payment provider")),
    };

    // TODO: Parse webhook based on provider format
    let webhook_data = PaymentWebhook {
        provider: payment_provider,
        provider_reference: webhook["reference"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        status: crate::application::exchange::payment_processor::PaymentStatus::Success,
        amount: webhook["amount"]
            .as_f64()
            .map(|a| rust_decimal::Decimal::from_f64_retain(a).unwrap_or_default())
            .unwrap_or_default(),
        timestamp: chrono::Utc::now(),
        signature: webhook["signature"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        raw_data: webhook,
    };

    processor.handle_webhook(webhook_data).await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}

// POST /api/v1/exchange/expired/process
pub async fn process_expired_transactions(
    Extension(user): Extension<CurrentUser>,
    State(service): State<std::sync::Arc<TransactionService>>,
) -> AppResult<impl IntoResponse> {
    // Only admins can trigger this
    if !user.is_admin() {
        return Err(AppError::forbidden("Admin access required"));
    }

    let count = service.process_expired_transactions().await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "processed": count,
        "status": "completed"
    }))))
}