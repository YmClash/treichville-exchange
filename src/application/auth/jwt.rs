use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    shared::config::JwtConfig,
    domain::user::{User, UserRole},
    shared::errors::AppError,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,        // User ID
    pub exp: i64,           // Expiration time
    pub iat: i64,           // Issued at
    pub nbf: i64,           // Not before
    pub iss: String,        // Issuer
    pub aud: String,        // Audience
    pub jti: String,        // JWT ID
    pub role: UserRole,     // User role
    pub phone: String,      // User phone
}

impl JwtClaims {
    pub fn new(user: &User, config: &JwtConfig, is_refresh: bool) -> Self {
        let now = Utc::now();
        let expiry_duration = if is_refresh {
            Duration::seconds(config.refresh_token_expiry)
        } else {
            Duration::seconds(config.access_token_expiry)
        };
        let exp = (now + expiry_duration).timestamp();

        Self {
            sub: user.id.to_string(),
            exp,
            iat: now.timestamp(),
            nbf: now.timestamp(),
            iss: config.issuer.clone(),
            aud: if is_refresh { "refresh".to_string() } else { "access".to_string() },
            jti: Uuid::new_v4().to_string(),
            role: user.role.clone(),
            phone: user.phone.clone(),
        }
    }

    pub fn user_id(&self) -> Result<Uuid, AppError> {
        Uuid::parse_str(&self.sub)
            .map_err(|_| AppError::authentication("Invalid user ID in token"))
    }

    pub fn is_expired(&self) -> bool {
        self.exp < Utc::now().timestamp()
    }

    pub fn is_refresh_token(&self) -> bool {
        self.aud == "refresh"
    }
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    config: JwtConfig,
}

impl JwtService {
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());
        
        let mut validation = Validation::default();
        validation.set_issuer(&[config.issuer.clone()]);
        validation.set_required_spec_claims(&["sub", "exp", "iat", "nbf", "iss", "aud", "jti"]);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = 5; // 5 seconds leeway for clock skew

        Self {
            encoding_key,
            decoding_key,
            validation,
            config,
        }
    }

    pub fn generate_access_token(&self, user: &User) -> Result<String, AppError> {
        let claims = JwtClaims::new(user, &self.config, false);
        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::authentication(format!("Failed to generate access token: {}", e)))
    }

    pub fn generate_refresh_token(&self, user: &User) -> Result<String, AppError> {
        let claims = JwtClaims::new(user, &self.config, true);
        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::authentication(format!("Failed to generate refresh token: {}", e)))
    }

    pub fn verify_token(&self, token: &str) -> Result<JwtClaims, AppError> {
        let mut validation = self.validation.clone();
        validation.set_audience(&["access", "refresh"]);
        
        let token_data: TokenData<JwtClaims> = decode(token, &self.decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    AppError::authentication("Token has expired")
                }
                jsonwebtoken::errors::ErrorKind::InvalidToken => {
                    AppError::authentication("Invalid token")
                }
                jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                    AppError::authentication("Invalid token signature")
                }
                _ => AppError::authentication(format!("Token validation failed: {}", e)),
            })?;

        Ok(token_data.claims)
    }

    pub fn verify_access_token(&self, token: &str) -> Result<JwtClaims, AppError> {
        let claims = self.verify_token(token)?;
        
        if claims.is_refresh_token() {
            return Err(AppError::authentication("Invalid token type"));
        }

        Ok(claims)
    }

    pub fn verify_refresh_token(&self, token: &str) -> Result<JwtClaims, AppError> {
        let claims = self.verify_token(token)?;
        
        if !claims.is_refresh_token() {
            return Err(AppError::authentication("Invalid token type"));
        }

        Ok(claims)
    }

    pub fn extract_token_from_header(auth_header: &str) -> Result<String, AppError> {
        let parts: Vec<&str> = auth_header.split_whitespace().collect();
        
        if parts.len() != 2 || parts[0] != "Bearer" {
            return Err(AppError::authentication("Invalid authorization header format"));
        }

        Ok(parts[1].to_string())
    }
}