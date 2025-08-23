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
        rates::RateService,
    },
    domain::{
        rate::{GetQuoteRequest, ExchangeOperation, CreateRateDto, UpdateRateDto},
        user::UserRole,
    },
    shared::{
        errors::{AppError, AppResult},
        utils::PaginationParams,
        state::AppState,
    },
};

#[derive(Debug, Deserialize)]
pub struct GetRatesQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetBestRateQuery {
    pub from: String,
    pub to: String,
    pub amount: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct GetQuoteQuery {
    pub from: String,
    pub to: String,
    pub amount: Decimal,
    pub operation: String, // "buy" or "sell"
}

// GET /api/v1/rates/public
pub async fn get_public_rates(
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let rates = state.rate_service.get_public_rates().await?;
    
    Ok((StatusCode::OK, Json(rates)))
}

// GET /api/v1/rates/best
pub async fn get_best_rate(
    Query(params): Query<GetBestRateQuery>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let response = state.rate_service
        .get_best_rates(&params.from, &params.to, params.amount)
        .await?;
    
    Ok((StatusCode::OK, Json(response)))
}

// GET /api/v1/rates/quote
pub async fn get_quote(
    Query(params): Query<GetQuoteQuery>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let operation = match params.operation.to_lowercase().as_str() {
        "buy" => ExchangeOperation::Buy,
        "sell" => ExchangeOperation::Sell,
        _ => return Err(AppError::validation("Invalid operation. Use 'buy' or 'sell'")),
    };

    let request = GetQuoteRequest {
        from_currency: params.from,
        to_currency: params.to,
        amount: params.amount,
        operation,
    };

    let quote = state.rate_service.get_quote(request).await?;
    
    Ok((StatusCode::OK, Json(quote)))
}

// GET /api/v1/rates
pub async fn get_rates(
    Query(params): Query<GetRatesQuery>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let rates = if let (Some(from), Some(to)) = (params.from, params.to) {
        state.rate_service.get_rates_by_pair(&from, &to).await?
    } else {
        state.rate_service.get_public_rates().await?
            .into_iter()
            .map(|_| vec![])
            .flatten()
            .collect()
    };
    
    Ok((StatusCode::OK, Json(rates)))
}

// GET /api/v1/rates/changeur
pub async fn get_changeur_rates(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    // Check if user is a changeur
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can access their rates"));
    }

    let rates = state.rate_service.get_changeur_rates(user.id).await?;
    
    Ok((StatusCode::OK, Json(rates)))
}

// POST /api/v1/rates
pub async fn create_rate(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(dto): Json<CreateRateDto>,
) -> AppResult<impl IntoResponse> {
    // Check if user is a changeur
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can create rates"));
    }

    let rate = state.rate_service.create_rate(user.id, dto).await?;
    
    Ok((StatusCode::CREATED, Json(rate)))
}

// PUT /api/v1/rates/:id
pub async fn update_rate(
    Path(id): Path<Uuid>,
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(dto): Json<UpdateRateDto>,
) -> AppResult<impl IntoResponse> {
    // Check if user is a changeur
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can update rates"));
    }

    let rate = state.rate_service.update_rate(id, user.id, dto).await?;
    
    Ok((StatusCode::OK, Json(rate)))
}

// DELETE /api/v1/rates/:id
pub async fn delete_rate(
    Path(id): Path<Uuid>,
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    // Check if user is a changeur
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can delete rates"));
    }

    state.rate_service.delete_rate(id, user.id).await?;
    
    Ok((StatusCode::NO_CONTENT, ()))
}

// POST /api/v1/rates/bulk
pub async fn bulk_update_rates(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(updates): Json<Vec<BulkRateUpdate>>,
) -> AppResult<impl IntoResponse> {
    // Check if user is a changeur
    if !user.is_changeur() {
        return Err(AppError::forbidden("Only changeurs can update rates"));
    }

    let rate_updates: Vec<(String, String, Decimal, Decimal)> = updates
        .into_iter()
        .map(|u| (u.from_currency, u.to_currency, u.buy_rate, u.sell_rate))
        .collect();

    let rates = state.rate_service.bulk_update_rates(user.id, rate_updates).await?;
    
    Ok((StatusCode::OK, Json(rates)))
}

#[derive(Debug, Deserialize)]
pub struct BulkRateUpdate {
    pub from_currency: String,
    pub to_currency: String,
    pub buy_rate: Decimal,
    pub sell_rate: Decimal,
}

// GET /api/v1/rates/market-depth
pub async fn get_market_depth(
    Query(params): Query<GetRatesQuery>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let (from, to) = match (params.from, params.to) {
        (Some(f), Some(t)) => (f, t),
        _ => return Err(AppError::validation("Both 'from' and 'to' currencies are required")),
    };

    let depth = state.rate_service.get_market_depth(&from, &to).await?;
    
    Ok((StatusCode::OK, Json(depth)))
}

// GET /api/v1/rates/aggregated
pub async fn get_aggregated_rates(
    Query(params): Query<GetRatesQuery>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let (from, to) = match (params.from, params.to) {
        (Some(f), Some(t)) => (f, t),
        _ => return Err(AppError::validation("Both 'from' and 'to' currencies are required")),
    };

    let aggregated = state.rate_service.get_aggregated_rates(&from, &to).await?;
    
    Ok((StatusCode::OK, Json(aggregated)))
}