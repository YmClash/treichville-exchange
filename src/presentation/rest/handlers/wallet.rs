use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
    Extension,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rust_decimal::Decimal;

use crate::{
    application::{
        auth::middleware::CurrentUser,
        exchange::WalletService,
    },
    domain::wallet::{WalletOperation, TransferRequest},
    shared::{
        errors::{AppError, AppResult},
        utils::PaginationParams,
    },
};

// GET /api/v1/wallet/balance
pub async fn get_balance(
    Extension(user): Extension<CurrentUser>,
    State(wallet_service): State<std::sync::Arc<WalletService>>,
) -> AppResult<impl IntoResponse> {
    let balance = wallet_service.get_balance(user.id).await?;
    
    Ok((StatusCode::OK, Json(balance)))
}

// GET /api/v1/wallet/balance/:currency
pub async fn get_balance_by_currency(
    Path(currency): Path<String>,
    Extension(user): Extension<CurrentUser>,
    State(wallet_service): State<std::sync::Arc<WalletService>>,
) -> AppResult<impl IntoResponse> {
    let balance = wallet_service
        .get_balance_by_currency(user.id, &currency)
        .await?;
    
    Ok((StatusCode::OK, Json(balance)))
}

// POST /api/v1/wallet/deposit
pub async fn deposit(
    Extension(user): Extension<CurrentUser>,
    State(wallet_service): State<std::sync::Arc<WalletService>>,
    Json(request): Json<DepositRequest>,
) -> AppResult<impl IntoResponse> {
    // Only changeurs can deposit to their wallets
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can manage wallet deposits"));
    }

    let operation = WalletOperation::Deposit {
        user_id: user.id,
        currency: request.currency,
        amount: request.amount,
        reference: request.reference,
    };

    let transaction = wallet_service.process_operation(operation).await?;
    
    Ok((StatusCode::CREATED, Json(transaction)))
}

// POST /api/v1/wallet/withdraw
pub async fn withdraw(
    Extension(user): Extension<CurrentUser>,
    State(wallet_service): State<std::sync::Arc<WalletService>>,
    Json(request): Json<WithdrawRequest>,
) -> AppResult<impl IntoResponse> {
    // Only changeurs can withdraw from their wallets
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can manage wallet withdrawals"));
    }

    let operation = WalletOperation::Withdraw {
        user_id: user.id,
        currency: request.currency,
        amount: request.amount,
        reference: request.reference,
    };

    let transaction = wallet_service.process_operation(operation).await?;
    
    Ok((StatusCode::CREATED, Json(transaction)))
}

// POST /api/v1/wallet/transfer
pub async fn transfer(
    Extension(user): Extension<CurrentUser>,
    State(wallet_service): State<std::sync::Arc<WalletService>>,
    Json(request): Json<TransferRequest>,
) -> AppResult<impl IntoResponse> {
    // Validate request
    if request.from_user_id != user.id {
        return Err(AppError::forbidden("You can only transfer from your own wallet"));
    }

    let transaction = wallet_service.transfer(request).await?;
    
    Ok((StatusCode::CREATED, Json(transaction)))
}

// POST /api/v1/wallet/reserve
pub async fn reserve_funds(
    Extension(user): Extension<CurrentUser>,
    State(wallet_service): State<std::sync::Arc<WalletService>>,
    Json(request): Json<ReserveRequest>,
) -> AppResult<impl IntoResponse> {
    // Only internal operations or changeurs can reserve funds
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can reserve funds"));
    }

    let operation = WalletOperation::Reserve {
        user_id: user.id,
        currency: request.currency,
        amount: request.amount,
        transaction_id: request.transaction_id,
    };

    let transaction = wallet_service.process_operation(operation).await?;
    
    Ok((StatusCode::OK, Json(transaction)))
}

// POST /api/v1/wallet/release
pub async fn release_funds(
    Extension(user): Extension<CurrentUser>,
    State(wallet_service): State<std::sync::Arc<WalletService>>,
    Json(request): Json<ReleaseRequest>,
) -> AppResult<impl IntoResponse> {
    // Only internal operations or changeurs can release funds
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can release funds"));
    }

    let operation = WalletOperation::Release {
        user_id: user.id,
        currency: request.currency,
        amount: request.amount,
        transaction_id: request.transaction_id,
    };

    let transaction = wallet_service.process_operation(operation).await?;
    
    Ok((StatusCode::OK, Json(transaction)))
}

// GET /api/v1/wallet/transactions
pub async fn get_wallet_transactions(
    Query(params): Query<WalletTransactionQuery>,
    Extension(user): Extension<CurrentUser>,
    State(wallet_service): State<std::sync::Arc<WalletService>>,
) -> AppResult<impl IntoResponse> {
    let transactions = wallet_service
        .get_transactions(
            user.id, 
            params.currency.as_deref(),
            params.operation_type.as_deref(),
            params.page.unwrap_or(1),
            params.per_page.unwrap_or(20),
        )
        .await?;
    
    Ok((StatusCode::OK, Json(transactions)))
}

// GET /api/v1/wallet/statistics
pub async fn get_wallet_statistics(
    Query(params): Query<WalletStatsQuery>,
    Extension(user): Extension<CurrentUser>,
    State(wallet_service): State<std::sync::Arc<WalletService>>,
) -> AppResult<impl IntoResponse> {
    let stats = wallet_service
        .get_statistics(
            user.id,
            params.period.as_deref().unwrap_or("month"),
        )
        .await?;
    
    Ok((StatusCode::OK, Json(stats)))
}

// POST /api/v1/wallet/reconcile
pub async fn reconcile_wallet(
    Extension(user): Extension<CurrentUser>,
    State(wallet_service): State<std::sync::Arc<WalletService>>,
    Json(request): Json<ReconcileRequest>,
) -> AppResult<impl IntoResponse> {
    // Only admins can reconcile wallets
    if !user.is_admin() {
        return Err(AppError::forbidden("Admin access required"));
    }

    let result = wallet_service
        .reconcile_wallet(request.user_id, &request.currency)
        .await?;
    
    Ok((StatusCode::OK, Json(result)))
}

// Request DTOs
#[derive(Debug, Deserialize)]
pub struct DepositRequest {
    pub currency: String,
    pub amount: Decimal,
    pub reference: String,
}

#[derive(Debug, Deserialize)]
pub struct WithdrawRequest {
    pub currency: String,
    pub amount: Decimal,
    pub reference: String,
}

#[derive(Debug, Deserialize)]
pub struct ReserveRequest {
    pub currency: String,
    pub amount: Decimal,
    pub transaction_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseRequest {
    pub currency: String,
    pub amount: Decimal,
    pub transaction_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct WalletTransactionQuery {
    pub currency: Option<String>,
    pub operation_type: Option<String>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct WalletStatsQuery {
    pub period: Option<String>, // "day", "week", "month", "year"
}

#[derive(Debug, Deserialize)]
pub struct ReconcileRequest {
    pub user_id: Uuid,
    pub currency: String,
}

// GET /api/v1/wallet/limits
pub async fn get_wallet_limits(
    Extension(user): Extension<CurrentUser>,
    State(wallet_service): State<std::sync::Arc<WalletService>>,
) -> AppResult<impl IntoResponse> {
    let limits = wallet_service.get_limits(user.id).await?;
    
    Ok((StatusCode::OK, Json(limits)))
}

// GET /api/v1/wallet/pending-operations
pub async fn get_pending_operations(
    Extension(user): Extension<CurrentUser>,
    State(wallet_service): State<std::sync::Arc<WalletService>>,
) -> AppResult<impl IntoResponse> {
    let operations = wallet_service
        .get_pending_operations(user.id)
        .await?;
    
    Ok((StatusCode::OK, Json(operations)))
}

// POST /api/v1/wallet/validate-operation
pub async fn validate_operation(
    Extension(user): Extension<CurrentUser>,
    State(wallet_service): State<std::sync::Arc<WalletService>>,
    Json(request): Json<ValidateOperationRequest>,
) -> AppResult<impl IntoResponse> {
    let validation = wallet_service
        .validate_operation(
            user.id,
            &request.operation_type,
            &request.currency,
            request.amount,
        )
        .await?;
    
    Ok((StatusCode::OK, Json(validation)))
}

#[derive(Debug, Deserialize)]
pub struct ValidateOperationRequest {
    pub operation_type: String,
    pub currency: String,
    pub amount: Decimal,
}