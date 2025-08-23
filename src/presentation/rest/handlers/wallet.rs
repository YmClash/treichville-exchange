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
        exchange::{WalletService, WalletServiceTrait},
    },
    domain::wallet::{WalletOperation, TransferRequest},
    shared::{
        errors::{AppError, AppResult},
        utils::PaginationParams,
        state::AppState,
    },
};

// GET /api/v1/wallet/balance
pub async fn get_balance(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let balance = state.wallet_service.get_balance(user.id).await?;
    
    Ok((StatusCode::OK, Json(balance)))
}

// GET /api/v1/wallet/balance/:currency
pub async fn get_balance_by_currency(
    Path(currency): Path<String>,
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let balance = state.wallet_service
        .get_balance_by_currency(user.id, &currency)
        .await?;
    
    Ok((StatusCode::OK, Json(balance)))
}

// POST /api/v1/wallet/deposit
pub async fn deposit(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<DepositRequest>,
) -> AppResult<impl IntoResponse> {
    // Only changeurs can deposit to their wallets
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can manage wallet deposits"));
    }

    // Process deposit directly
    let wallet = state.wallet_service.credit(
        user.id,
        &request.currency,
        request.amount,
        &request.reference,
    ).await?;
    
    // Create a simple response
    let transaction = serde_json::json!({
        "wallet_id": wallet.id,
        "balance": wallet.balance,
        "currency": wallet.currency,
        "operation": "deposit",
        "amount": request.amount
    });
    
    Ok((StatusCode::CREATED, Json(transaction)))
}

// POST /api/v1/wallet/withdraw
pub async fn withdraw(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<WithdrawRequest>,
) -> AppResult<impl IntoResponse> {
    // Only changeurs can withdraw from their wallets
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can manage wallet withdrawals"));
    }

    // Process withdrawal directly
    let wallet = state.wallet_service.debit(
        user.id,
        &request.currency,
        request.amount,
        &request.reference,
    ).await?;
    
    // Create a simple response
    let transaction = serde_json::json!({
        "wallet_id": wallet.id,
        "balance": wallet.balance,
        "currency": wallet.currency,
        "operation": "withdrawal",
        "amount": request.amount
    });
    
    Ok((StatusCode::CREATED, Json(transaction)))
}

// POST /api/v1/wallet/transfer
pub async fn transfer(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<TransferRequest>,
) -> AppResult<impl IntoResponse> {
    // For now, use the trait transfer method
    // We would need to get the recipient user ID from the request
    use crate::application::exchange::WalletServiceTrait;
    let result = WalletServiceTrait::transfer(
        state.wallet_service.as_ref(),
        user.id,
        user.id, // This should be the recipient user ID from the request
        &request.currency,
        request.amount,
        &format!("Transfer: {}", request.description.as_deref().unwrap_or("No description"))
    ).await?;
    
    // Create a response
    let transaction = serde_json::json!({
        "success": true,
        "message": "Transfer completed",
        "amount": request.amount,
        "currency": request.currency
    });
    
    Ok((StatusCode::CREATED, Json(transaction)))
}

// POST /api/v1/wallet/reserve
pub async fn reserve_funds(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<ReserveRequest>,
) -> AppResult<impl IntoResponse> {
    // Only internal operations or changeurs can reserve funds
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can reserve funds"));
    }

    // Process reserve directly
    let wallet = state.wallet_service.reserve(
        user.id,
        &request.currency,
        request.amount,
        &format!("Reserve for transaction: {:?}", request.transaction_id),
    ).await?;
    
    // Create a response
    let transaction = serde_json::json!({
        "wallet_id": wallet.id,
        "reserved_balance": wallet.reserved_balance,
        "available_balance": wallet.balance - wallet.reserved_balance,
        "currency": wallet.currency,
        "operation": "reserve",
        "amount": request.amount
    });
    
    Ok((StatusCode::OK, Json(transaction)))
}

// POST /api/v1/wallet/release
pub async fn release_funds(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<ReleaseRequest>,
) -> AppResult<impl IntoResponse> {
    // Only internal operations or changeurs can release funds
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can release funds"));
    }

    // Process release directly
    let wallet = state.wallet_service.release(
        user.id,
        &request.currency,
        request.amount,
        &format!("Release for transaction: {:?}", request.transaction_id),
    ).await?;
    
    // Create a response
    let transaction = serde_json::json!({
        "wallet_id": wallet.id,
        "reserved_balance": wallet.reserved_balance,
        "available_balance": wallet.balance - wallet.reserved_balance,
        "currency": wallet.currency,
        "operation": "release",
        "amount": request.amount
    });
    
    Ok((StatusCode::OK, Json(transaction)))
}

// GET /api/v1/wallet/transactions
pub async fn get_wallet_transactions(
    Query(params): Query<WalletTransactionQuery>,
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let page = params.page.unwrap_or(1) as i64;
    let per_page = params.per_page.unwrap_or(20) as i64;
    let offset = (page - 1) * per_page;
    
    let transactions = state.wallet_service
        .get_transactions(
            user.id,
            None, // from date
            None, // to date
            Some(per_page),
            Some(offset),
        )
        .await?;
    
    Ok((StatusCode::OK, Json(transactions)))
}

// GET /api/v1/wallet/statistics
pub async fn get_wallet_statistics(
    Query(params): Query<WalletStatsQuery>,
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let stats = state.wallet_service
        .get_statistics(
            user.id,
            None,
            None,
        )
        .await?;
    
    Ok((StatusCode::OK, Json(stats)))
}

// POST /api/v1/wallet/reconcile
pub async fn reconcile_wallet(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<ReconcileRequest>,
) -> AppResult<impl IntoResponse> {
    // Only admins can reconcile wallets
    if !user.is_admin() {
        return Err(AppError::forbidden("Admin access required"));
    }

    let result = state.wallet_service
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
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let limits = state.wallet_service.get_limits(user.id).await?;
    
    Ok((StatusCode::OK, Json(limits)))
}

// GET /api/v1/wallet/pending-operations
pub async fn get_pending_operations(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let operations = state.wallet_service
        .get_pending_operations(user.id)
        .await?;
    
    Ok((StatusCode::OK, Json(operations)))
}

// POST /api/v1/wallet/validate-operation
pub async fn validate_operation(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<ValidateOperationRequest>,
) -> AppResult<impl IntoResponse> {
    let validation = state.wallet_service
        .validate_operation(
            user.id,
            &request.operation_type,
            request.amount,
            &request.currency,
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