use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres};
use tracing::log::warn;
use uuid::Uuid;
use validator::Validate;

use crate::{
    application::auth::{jwt::JwtService, dto::{UpdateProfileDto, ChangePasswordDto, Enable2FADto, Verify2FADto, RegisterDto, LoginDto}},
    shared::config::Settings,
    domain::{user::{RefreshToken, User, UserRole}},
    shared::{errors::AppError, utils},
};

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(custom = "utils::validate_phone_number")]
    pub phone: String,
    
    #[validate(length(min = 2, max = 100))]
    pub name: String,
    
    #[validate(email)]
    pub email: Option<String>,
    
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    
    pub role: Option<UserRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(custom = "utils::validate_phone_number")]
    pub phone: String,
    
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    
    pub two_fa_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub tokens: TokenPair,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub phone: String,
    pub name: String,
    pub email: Option<String>,
    pub role: UserRole,
    pub is_verified: bool,
    pub two_fa_enabled: bool,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            phone: utils::mask_phone_number(&user.phone),
            name: user.name,
            email: user.email.as_ref().map(|e| utils::mask_email(e)),
            role: user.role,
            is_verified: user.is_verified,
            two_fa_enabled: user.two_fa_enabled,
        }
    }
}

pub struct AuthService {
    db: Pool<Postgres>,
    redis: redis::aio::ConnectionManager,
    jwt_service: JwtService,
    settings: Settings,
}

impl AuthService {
    pub fn new(
        db: Pool<Postgres>,
        redis: redis::aio::ConnectionManager,
        settings: Settings,
    ) -> Self {
        let jwt_service = JwtService::new(settings.jwt.clone());
        
        Self {
            db,
            redis,
            jwt_service,
            settings,
        }
    }

    pub async fn register(&self, dto: RegisterDto) -> Result<LoginResponse, AppError> {
        dto.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;
        
        let request = RegisterRequest {
            phone: dto.phone.clone(),
            name: dto.name.clone(),
            email: dto.email.clone(),
            password: dto.password.clone(),
            role: dto.role,
        };

        // Check if user already exists
        let existing = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE phone = $1 OR email = $2"
        )
        .bind(&request.phone)
        .bind(&request.email)
        .fetch_optional(&self.db)
        .await?;

        if existing.is_some() {
            return Err(AppError::conflict("User with this phone or email already exists"));
        }

        // Hash password
        let password_hash = self.hash_password(&request.password)?;

        // Create user
        let role = request.role.unwrap_or(UserRole::Client);
        let user = User::new(request.phone, request.name, password_hash, role);

        // Insert into database
        let saved_user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (
                id, phone, email, name, password_hash, role, 
                kyc_status, daily_limit, monthly_limit, is_active, 
                is_verified, two_fa_enabled
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#
        )
        .bind(user.id)
        .bind(&user.phone)
        .bind(&request.email)
        .bind(&user.name)
        .bind(&user.password_hash)
        .bind(&user.role)
        .bind(&user.kyc_status)
        .bind(user.daily_limit)
        .bind(user.monthly_limit)
        .bind(user.is_active)
        .bind(user.is_verified)
        .bind(user.two_fa_enabled)
        .fetch_one(&self.db)
        .await?;

        // Generate tokens
        let tokens = self.generate_token_pair(&saved_user).await?;

        // Log registration
        self.log_auth_event(&saved_user.id, "register").await?;

        Ok(LoginResponse {
            user: saved_user.clone().into(),
            tokens,
        })
    }

    pub async fn login(&self, dto: LoginDto) -> Result<LoginResponse, AppError> {
        dto.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;
        
        let request = LoginRequest {
            phone: dto.phone.clone(),
            password: dto.password.clone(),
            two_fa_code: dto.two_fa_code,
        };

        // Find user
        let mut user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE phone = $1"
        )
        .bind(&request.phone)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::authentication("Invalid credentials"))?;

        // Check if account is locked
        if user.is_locked() {
            return Err(AppError::authentication("Account is temporarily locked"));
        }

        // Check if account is active
        if !user.is_active {
            return Err(AppError::authentication("Account is disabled"));
        }

        // Verify password
        if !self.verify_password(&request.password, &user.password_hash)? {
            // Increment failed attempts
            self.handle_failed_login(&mut user).await?;
            return Err(AppError::authentication("Invalid credentials"));
        }

        // Verify 2FA if enabled
        if user.two_fa_enabled {
            if request.two_fa_code.is_none() {
                return Err(AppError::authentication("2FA code required"));
            }
            // TODO: Implement actual 2FA verification
            warn!("2FA verification not implemented");
            // For now, we'll skip the actual verification
        }

        // Reset failed attempts and update last login
        self.handle_successful_login(&mut user).await?;

        // Generate tokens
        let tokens = self.generate_token_pair(&user).await?;

        // Log login
        self.log_auth_event(&user.id, "login").await?;

        Ok(LoginResponse {
            user: user.into(),
            tokens,
        })
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenPair, AppError> {
        // Verify refresh token
        let claims = self.jwt_service.verify_refresh_token(refresh_token)?;
        let user_id = claims.user_id()?;

        // Check if refresh token exists and is valid
        let token_hash = self.hash_token(refresh_token);
        let stored_token = sqlx::query_as::<_, RefreshToken>(
            "SELECT * FROM refresh_tokens WHERE user_id = $1 AND token_hash = $2"
        )
        .bind(user_id)
        .bind(&token_hash)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::authentication("Invalid refresh token"))?;

        if !stored_token.is_valid() {
            return Err(AppError::authentication("Refresh token has expired or been revoked"));
        }

        // Get user
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

        // Revoke old refresh token
        sqlx::query(
            "UPDATE refresh_tokens SET revoked = true, revoked_at = NOW() WHERE id = $1"
        )
        .bind(stored_token.id)
        .execute(&self.db)
        .await?;

        // Generate new token pair
        let tokens = self.generate_token_pair(&user).await?;

        Ok(tokens)
    }

    pub async fn logout(&self, user_id: Uuid, access_token: &str) -> Result<(), AppError> {
        // Revoke all refresh tokens for the user
        sqlx::query(
            "UPDATE refresh_tokens SET revoked = true, revoked_at = NOW() 
             WHERE user_id = $1 AND revoked = false"
        )
        .bind(user_id)
        .execute(&self.db)
        .await?;

        // Blacklist access token in Redis until it expires
        let token_hash = self.hash_token(access_token);
        let key = format!("blacklist:{}", token_hash);
        let ttl = self.settings.jwt.access_token_expiry as u64;
        
        let _: () = self.redis.clone()
            .set_ex(key, "1", ttl)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

        // Log logout
        self.log_auth_event(&user_id, "logout").await?;

        Ok(())
    }

    pub async fn is_token_blacklisted(&self, token: &str) -> Result<bool, AppError> {
        let token_hash = self.hash_token(token);
        let key = format!("blacklist:{}", token_hash);
        
        let exists: bool = self.redis.clone()
            .exists(key)
            .await
            .map_err(|e| AppError::service_unavailable(format!("Redis error: {}", e)))?;

        Ok(exists)
    }

    async fn generate_token_pair(&self, user: &User) -> Result<TokenPair, AppError> {
        let access_token = self.jwt_service.generate_access_token(user)?;
        let refresh_token = self.jwt_service.generate_refresh_token(user)?;

        // Store refresh token
        let token_hash = self.hash_token(&refresh_token);
        let expires_at = Utc::now() + Duration::seconds(self.settings.jwt.refresh_token_expiry);

        sqlx::query(
            r#"
            INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at)
            VALUES ($1, $2, $3, $4)
            "#
        )
        .bind(Uuid::new_v4())
        .bind(user.id)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&self.db)
        .await?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.settings.jwt.access_token_expiry,
        })
    }

    fn hash_password(&self, password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::InternalServerError)?
            .to_string();

        Ok(password_hash)
    }

    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AppError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|_| AppError::InternalServerError)?;
        
        let argon2 = Argon2::default();
        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    async fn handle_failed_login(&self, user: &mut User) -> Result<(), AppError> {
        user.failed_login_attempts += 1;

        // Lock account after 5 failed attempts
        if user.failed_login_attempts >= 5 {
            user.locked_until = Some(Utc::now() + Duration::minutes(15));
        }

        sqlx::query(
            r#"
            UPDATE users 
            SET failed_login_attempts = $1, locked_until = $2, updated_at = NOW()
            WHERE id = $3
            "#
        )
        .bind(user.failed_login_attempts)
        .bind(user.locked_until)
        .bind(user.id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn handle_successful_login(&self, user: &mut User) -> Result<(), AppError> {
        user.failed_login_attempts = 0;
        user.locked_until = None;
        user.last_login_at = Some(Utc::now());

        sqlx::query(
            r#"
            UPDATE users 
            SET failed_login_attempts = 0, locked_until = NULL, 
                last_login_at = NOW(), updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(user.id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn log_auth_event(&self, user_id: &Uuid, action: &str) -> Result<(), AppError> {
        // TODO: Implement audit logging

        // For now, just log to tracing
        tracing::info!(user_id = %user_id, action = action, "Auth event");
        Ok(())
    }

    // Additional methods required by the handlers
    pub async fn get_user_details(&self, user_id: Uuid) -> Result<User, AppError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.db)
            .await
            .map_err(|_| AppError::not_found("User"))
    }
    
    pub async fn update_profile(&self, user_id: Uuid, dto: UpdateProfileDto) -> Result<User, AppError> {
        // Build update query dynamically based on provided fields
        let mut query = "UPDATE users SET updated_at = NOW()".to_string();
        let mut params: Vec<String> = vec![];
        
        if dto.name.is_some() {
            params.push("name = $2".to_string());
        }
        if dto.email.is_some() {
            params.push("email = $3".to_string());
        }
        if dto.address.is_some() {
            params.push("address = $4".to_string());
        }
        
        if !params.is_empty() {
            query.push_str(", ");
            query.push_str(&params.join(", "));
        }
        
        query.push_str(" WHERE id = $1 RETURNING *");
        
        // For now, simple implementation
        sqlx::query_as::<_, User>(&query)
            .bind(user_id)
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Database(e))
    }
    
    pub async fn change_password(&self, user_id: Uuid, dto: ChangePasswordDto) -> Result<(), AppError> {
        // Get user
        let user = self.get_user_details(user_id).await?;
        
        // Verify current password
        self.verify_password(&dto.current_password, &user.password_hash)?;
        
        // Hash new password
        let new_hash = self.hash_password(&dto.new_password)?;
        
        // Update password
        sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_hash)
            .bind(user_id)
            .execute(&self.db)
            .await?;
        
        Ok(())
    }
    
    pub async fn verify_email(&self, token: &str) -> Result<(), AppError> {
        // TODO: Implement email verification with token
        todo!("Implement verify_email")
    }
    
    pub async fn initiate_password_reset(&self, email: &str) -> Result<(), AppError> {
        // TODO: Generate reset token and send email
        todo!("Implement initiate_password_reset")
    }
    
    pub async fn reset_password(&self, token: String, new_password: String) -> Result<(), AppError> {
        // TODO: Verify token and reset password
        todo!("Implement reset_password")
    }
    
    pub async fn enable_2fa(&self, user_id: Uuid, dto: Enable2FADto) -> Result<String, AppError> {
        // TODO: Generate and store 2FA secret
        todo!("Implement enable_2fa code")
        // warn!("2FA enabling not implemented");


    }
    
    pub async fn verify_2fa(&self, user_id: Uuid, dto: Verify2FADto) -> Result<(), AppError> {
        // todo!("Implement verify_2fa code")
        todo!("Implement verify_2fa code")
        // warn!("2FA verification not implemented");
    }
    
    pub async fn disable_2fa(&self, user_id: Uuid, password: String) -> Result<(), AppError> {
        // TODO: Disable 2FA after password verification
        todo!("Implement disable_2fa code")
        // warn!("2FA disabling not implemented");
    }
    
    pub async fn get_users(&self, page: Option<u32>, limit: Option<u32>, role: Option<String>, active: Option<bool>) -> Result<Vec<User>, AppError> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;
        
        let mut query = "SELECT * FROM users WHERE 1=1".to_string();
        
        if let Some(role) = role {
            query.push_str(&format!(" AND role = '{}'", role));
        }
        
        if let Some(active) = active {
            query.push_str(&format!(" AND is_active = {}", active));
        }
        
        query.push_str(&format!(" ORDER BY created_at DESC LIMIT {} OFFSET {}", limit, offset));
        
        sqlx::query_as::<_, User>(&query)
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::Database(e))
    }
    
    pub async fn activate_user(&self, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET is_active = true, updated_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }
    
    pub async fn deactivate_user(&self, user_id: Uuid, reason: String) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET is_active = false, deactivation_reason = $1, updated_at = NOW() WHERE id = $2")
            .bind(reason)
            .bind(user_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }
    
    pub async fn verify_changeur(&self, user_id: Uuid, verification_type: String, documents: Vec<String>) -> Result<(), AppError> {
        //Implement changeur verification

        let documents_json = serde_json::to_value(documents)
            .map_err(|e| AppError::validation(format!("Invalid documents format: {}", e)))?;
        
        sqlx::query("UPDATE users SET is_verified = true, verification_type = $1, verification_documents = $2, verified_at = NOW() WHERE id = $3")
            .bind(verification_type)
            .bind(documents_json)
            .bind(user_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }
    
    pub async fn get_active_sessions(&self, user_id: Uuid) -> Result<Vec<String>, AppError> {
        // TODO: Implement session tracking
        // warn!("Session tracking not implemented");
        Ok(vec![])
    }
    
    pub async fn revoke_session(&self, user_id: Uuid, session_id: String) -> Result<(), AppError> {
        // TODO: Implement session revocation
        warn!("Session revocation not implemented");
        Ok(())
    }
    
    pub async fn revoke_all_sessions(&self, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
            .bind(user_id)

            .execute(&self.db)
            .await?;
        Ok(())
    }
}

// Implement conversions from DTOs to Requests
impl From<RegisterDto> for RegisterRequest {
    fn from(dto: RegisterDto) -> Self {
        RegisterRequest {
            phone: dto.phone,
            name: dto.name,
            email: dto.email,
            password: dto.password,
            role: dto.role,
        }
    }
}

impl From<LoginDto> for LoginRequest {
    fn from(dto: LoginDto) -> Self {
        LoginRequest {
            phone: dto.phone,
            password: dto.password,
            two_fa_code: dto.two_fa_code,
        }
    }
}