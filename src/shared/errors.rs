use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Unauthorized access")]
    Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Payment error: {0}")]
    Payment(String),

    #[error("External API error: {0}")]
    ExternalApi(String),

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("KYC limit exceeded")]
    KycLimitExceeded,

    #[error("Invalid currency: {0}")]
    InvalidCurrency(String),

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Transaction timeout")]
    TransactionTimeout,

    #[error("Duplicate transaction")]
    DuplicateTransaction,

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Internal server error")]
    InternalServerError,

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Argon2 error: {0}")]
    Argon2(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
    pub request_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    pub error_type: ErrorType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorType {
    ValidationError,
    AuthenticationError,
    AuthorizationError,
    NotFoundError,
    ConflictError,
    RateLimitError,
    TransactionError,
    PaymentError,
    ExternalApiError,
    InsufficientFundsError,
    KycLimitError,
    TimeoutError,
    DuplicateError,
    ServiceError,
    InternalError,
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(err: argon2::password_hash::Error) -> Self {
        AppError::Argon2(err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match &self {
            AppError::Database(e) => {
                error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorType::InternalError,
                    "Database operation failed".to_string(),
                )
            }
            AppError::Redis(e) => {
                error!("Redis error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorType::InternalError,
                    "Cache operation failed".to_string(),
                )
            }
            AppError::Validation(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorType::ValidationError,
                msg.clone(),
            ),
            AppError::Authentication(msg) => (
                StatusCode::UNAUTHORIZED,
                ErrorType::AuthenticationError,
                msg.clone(),
            ),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                ErrorType::AuthenticationError,
                "Unauthorized access".to_string(),
            ),
            AppError::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                ErrorType::AuthorizationError,
                msg.clone(),
            ),
            AppError::NotFound(resource) => (
                StatusCode::NOT_FOUND,
                ErrorType::NotFoundError,
                format!("{} not found", resource),
            ),
            AppError::Conflict(msg) => (
                StatusCode::CONFLICT,
                ErrorType::ConflictError,
                msg.clone(),
            ),
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorType::ValidationError,
                msg.clone(),
            ),
            AppError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                ErrorType::RateLimitError,
                "Rate limit exceeded. Please try again later".to_string(),
            ),
            AppError::Transaction(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorType::TransactionError,
                msg.clone(),
            ),
            AppError::Payment(msg) => (
                StatusCode::PAYMENT_REQUIRED,
                ErrorType::PaymentError,
                msg.clone(),
            ),
            AppError::ExternalApi(msg) => (
                StatusCode::BAD_GATEWAY,
                ErrorType::ExternalApiError,
                msg.clone(),
            ),
            AppError::InsufficientFunds => (
                StatusCode::PAYMENT_REQUIRED,
                ErrorType::InsufficientFundsError,
                "Insufficient funds for this transaction".to_string(),
            ),
            AppError::KycLimitExceeded => (
                StatusCode::FORBIDDEN,
                ErrorType::KycLimitError,
                "KYC limit exceeded. Please upgrade your account".to_string(),
            ),
            AppError::InvalidCurrency(currency) => (
                StatusCode::BAD_REQUEST,
                ErrorType::ValidationError,
                format!("Invalid currency: {}", currency),
            ),
            AppError::InvalidAmount(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorType::ValidationError,
                format!("Invalid amount: {}", msg),
            ),
            AppError::TransactionTimeout => (
                StatusCode::REQUEST_TIMEOUT,
                ErrorType::TimeoutError,
                "Transaction timeout. Please try again".to_string(),
            ),
            AppError::DuplicateTransaction => (
                StatusCode::CONFLICT,
                ErrorType::DuplicateError,
                "Duplicate transaction detected".to_string(),
            ),
            AppError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                ErrorType::ServiceError,
                msg.clone(),
            ),
            AppError::InternalServerError => {
                error!("Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorType::InternalError,
                    "Internal server error".to_string(),
                )
            }
            AppError::Jwt(e) => {
                error!("JWT error: {:?}", e);
                (
                    StatusCode::UNAUTHORIZED,
                    ErrorType::AuthenticationError,
                    "Invalid or expired token".to_string(),
                )
            }
            AppError::Argon2(e) => {
                error!("Password hashing error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorType::InternalError,
                    "Password processing failed".to_string(),
                )
            }
            AppError::Configuration(msg) => {
                error!("Configuration error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorType::InternalError,
                    "Configuration error".to_string(),
                )
            }
            AppError::Serialization(e) => {
                error!("Serialization error: {:?}", e);
                (
                    StatusCode::BAD_REQUEST,
                    ErrorType::ValidationError,
                    "Invalid data format".to_string(),
                )
            }
            AppError::Io(e) => {
                error!("IO error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorType::InternalError,
                    "IO operation failed".to_string(),
                )
            }
        };

        let error_response = ErrorResponse {
            error: ErrorDetail {
                code: format!("{:?}", error_type).to_uppercase(),
                message,
                error_type,
                details: None,
            },
            request_id: None,
            timestamp: chrono::Utc::now(),
        };

        (status, Json(error_response)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn authentication(msg: impl Into<String>) -> Self {
        Self::Authentication(msg.into())
    }

    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound(resource.into())
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::Forbidden(msg.into())
    }

    pub fn transaction(msg: impl Into<String>) -> Self {
        Self::Transaction(msg.into())
    }

    pub fn payment(msg: impl Into<String>) -> Self {
        Self::Payment(msg.into())
    }

    pub fn external_api(msg: impl Into<String>) -> Self {
        Self::ExternalApi(msg.into())
    }

    pub fn service_unavailable(msg: impl Into<String>) -> Self {
        Self::ServiceUnavailable(msg.into())
    }
}