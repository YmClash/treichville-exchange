use serde::{Deserialize, Serialize};
use validator::Validate;
use uuid::Uuid;
use crate::domain::user::{User, UserRole};

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterDto {
    #[validate(length(min = 8, max = 20))]
    pub phone: String,
    #[validate(length(min = 2, max = 100))]
    pub name: String,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(min = 6))]
    pub password: String,
    pub role: UserRole,
}

#[derive(Debug, Deserialize)]
pub struct LoginDto {
    pub phone: String,
    pub password: String,
    pub two_fa_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenDto {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordDto {
    pub current_password: String,
    #[validate(length(min = 6))]
    pub new_password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileDto {
    #[validate(length(min = 2, max = 100))]
    pub name: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Enable2FADto {
    pub method: String, // "totp" or "sms"
}

#[derive(Debug, Deserialize)]
pub struct Verify2FADto {
    pub code: String,
}

// Types de réponse
#[derive(Debug, Serialize)]
pub struct RegisterRequest {
    pub phone: String,
    pub name: String,
    pub email: Option<String>,
    pub password: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub phone: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: User,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

// Conversion des DTOs
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
        }
    }
}