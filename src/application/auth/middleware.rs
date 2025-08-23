use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use axum_extra::headers::{authorization::Bearer, Authorization};
use redis::AsyncCommands;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::auth::jwt::{JwtClaims, JwtService},
    shared::config::Settings,
    domain::user::{User, UserRole},
    shared::errors::AppError,
};

#[derive(Clone)]
pub struct AuthMiddleware {
    jwt_service: Arc<JwtService>,
    redis: redis::aio::ConnectionManager,
    db: Pool<Postgres>,
}

impl AuthMiddleware {
    pub fn new(
        settings: Settings,
        redis: redis::aio::ConnectionManager,
        db: Pool<Postgres>,
    ) -> Self {
        let jwt_service = Arc::new(JwtService::new(settings.jwt));
        
        Self {
            jwt_service,
            redis,
            db,
        }
    }

    pub async fn verify_token(&self, token: &str) -> Result<JwtClaims, AppError> {
        // Check if token is blacklisted
        let token_hash = self.hash_token(token);
        let key = format!("blacklist:{}", token_hash);
        
        let exists: bool = self.redis.clone()
            .exists(&key)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

        if exists {
            return Err(AppError::authentication("Token has been revoked"));
        }

        // Verify token
        let claims = self.jwt_service.verify_access_token(token)?;

        // Check if user still exists and is active
        let user_id = claims.user_id()?;
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::not_found("User"))?;

        if !user.is_active {
            return Err(AppError::authentication("Account is disabled"));
        }

        if user.is_locked() {
            return Err(AppError::authentication("Account is temporarily locked"));
        }

        Ok(claims)
    }

    fn hash_token(&self, token: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

pub async fn require_auth(
    State(auth): State<AuthMiddleware>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract token from header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::authentication("Missing authorization header"))?;

    let token = JwtService::extract_token_from_header(auth_header)?;

    // Verify token
    let claims = auth.verify_token(&token).await?;

    // Insert claims into request extensions
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

pub async fn require_role(
    allowed_roles: Vec<UserRole>,
) -> impl Fn(Request, Next) -> futures::future::BoxFuture<'static, Result<Response, AppError>> {
    move |mut request: Request, next: Next| {
        let allowed_roles = allowed_roles.clone();
        Box::pin(async move {
            // Get claims from request extensions
            let claims = request
                .extensions()
                .get::<JwtClaims>()
                .ok_or_else(|| AppError::Unauthorized)?
                .clone();

            // Check role
            if !allowed_roles.contains(&claims.role) {
                return Err(AppError::forbidden("Insufficient permissions"));
            }

            Ok(next.run(request).await)
        })
    }
}

pub async fn require_admin(
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let claims = request
        .extensions()
        .get::<JwtClaims>()
        .ok_or_else(|| AppError::Unauthorized)?
        .clone();

    if !matches!(claims.role, UserRole::Admin | UserRole::SuperAdmin) {
        return Err(AppError::forbidden("Admin access required"));
    }

    Ok(next.run(request).await)
}

pub async fn require_changeur(
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let claims = request
        .extensions()
        .get::<JwtClaims>()
        .ok_or_else(|| AppError::Unauthorized)?
        .clone();

    if !matches!(claims.role, UserRole::Changeur | UserRole::Admin | UserRole::SuperAdmin) {
        return Err(AppError::forbidden("Changeur access required"));
    }

    Ok(next.run(request).await)
}

pub async fn optional_auth(
    State(auth): State<AuthMiddleware>,
    mut request: Request,
    next: Next,
) -> Response {
    // Try to extract token from header
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Ok(token) = JwtService::extract_token_from_header(auth_str) {
                // Try to verify token, but don't fail if invalid
                if let Ok(claims) = auth.verify_token(&token).await {
                    request.extensions_mut().insert(claims);
                }
            }
        }
    }

    next.run(request).await
}

#[derive(Clone)]
pub struct CurrentUser {
    pub id: Uuid,
    pub phone: String,
    pub role: UserRole,
}

impl CurrentUser {
    pub fn from_request(request: &Request) -> Result<Self, AppError> {
        let claims = request
            .extensions()
            .get::<JwtClaims>()
            .ok_or_else(|| AppError::Unauthorized)?;

        Ok(Self {
            id: claims.user_id()?,
            phone: claims.phone.clone(),
            role: claims.role.clone(),
        })
    }

    pub fn is_admin(&self) -> bool {
        matches!(self.role, UserRole::Admin | UserRole::SuperAdmin)
    }

    pub fn is_changeur(&self) -> bool {
        matches!(self.role, UserRole::Changeur)
    }
}