use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Client,
    Changeur,
    Admin,
    SuperAdmin,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "kyc_level", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum KycLevel {
    None,
    Level0,
    Level1,
    Level2,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub phone: String,
    pub email: Option<String>,
    pub name: String,
    #[serde(skip)]
    pub password_hash: String,
    pub role: UserRole,
    pub kyc_status: KycLevel,
    pub daily_limit: Decimal,
    pub monthly_limit: Decimal,
    pub is_active: bool,
    pub is_verified: bool,
    pub two_fa_enabled: bool,
    #[serde(skip)]
    pub two_fa_secret: Option<String>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(phone: String, name: String, password_hash: String, role: UserRole) -> Self {
        let (daily_limit, monthly_limit) = match role {
            UserRole::Client => (Decimal::from(50000), Decimal::from(500000)),
            UserRole::Changeur => (Decimal::from(10000000), Decimal::from(100000000)),
            _ => (Decimal::from(0), Decimal::from(0)),
        };

        Self {
            id: Uuid::new_v4(),
            phone,
            email: None,
            name,
            password_hash,
            role,
            kyc_status: KycLevel::None,
            daily_limit,
            monthly_limit,
            is_active: true,
            is_verified: false,
            two_fa_enabled: false,
            two_fa_secret: None,
            last_login_at: None,
            failed_login_attempts: 0,
            locked_until: None,
            metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            locked_until > Utc::now()
        } else {
            false
        }
    }

    pub fn can_transact(&self, amount: Decimal) -> bool {
        self.is_active && !self.is_locked() && amount <= self.daily_limit
    }

    pub fn update_kyc_limits(&mut self) {
        let (daily, monthly) = match self.kyc_status {
            KycLevel::None => (Decimal::from(50000), Decimal::from(500000)),
            KycLevel::Level0 => (Decimal::from(50000), Decimal::from(500000)),
            KycLevel::Level1 => (Decimal::from(500000), Decimal::from(5000000)),
            KycLevel::Level2 => (Decimal::from(5000000), Decimal::from(50000000)),
        };
        self.daily_limit = daily;
        self.monthly_limit = monthly;
    }

    pub fn is_admin(&self) -> bool {
        matches!(self.role, UserRole::Admin | UserRole::SuperAdmin)
    }

    pub fn is_changeur(&self) -> bool {
        matches!(self.role, UserRole::Changeur)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChangeurProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub business_name: Option<String>,
    pub business_address: Option<String>,
    pub business_phone: Option<String>,
    pub license_number: Option<String>,
    pub rating: Decimal,
    pub total_transactions: i32,
    pub total_volume: Decimal,
    pub commission_rate: Decimal,
    pub available_currencies: Vec<String>,
    pub operating_hours: Option<serde_json::Value>,
    pub location_latitude: Option<Decimal>,
    pub location_longitude: Option<Decimal>,
    pub is_verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ChangeurProfile {
    pub fn new(user_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            business_name: None,
            business_address: None,
            business_phone: None,
            license_number: None,
            rating: Decimal::from(0),
            total_transactions: 0,
            total_volume: Decimal::from(0),
            commission_rate: Decimal::new(5, 3), // 0.005 = 0.5%
            available_currencies: vec!["XOF".to_string(), "EUR".to_string(), "USD".to_string()],
            operating_hours: None,
            location_latitude: None,
            location_longitude: None,
            is_verified: false,
            verified_at: None,
            metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn update_stats(&mut self, transaction_amount: Decimal) {
        self.total_transactions += 1;
        self.total_volume += transaction_amount;
    }

    pub fn calculate_commission(&self, amount: Decimal) -> Decimal {
        (amount * self.commission_rate).round_dp(2)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateUserDto {
    #[validate(custom = "crate::shared::utils::validate_phone_number")]
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
pub struct UpdateUserDto {
    #[validate(length(min = 2, max = 100))]
    pub name: Option<String>,
    
    #[validate(email)]
    pub email: Option<String>,
    
    pub two_fa_enabled: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub phone: String,
    pub email: Option<String>,
    pub name: String,
    pub role: UserRole,
    pub kyc_status: KycLevel,
    pub daily_limit: Decimal,
    pub monthly_limit: Decimal,
    pub is_active: bool,
    pub is_verified: bool,
    pub two_fa_enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            phone: crate::shared::utils::mask_phone_number(&user.phone),
            email: user.email.as_ref().map(|e| crate::shared::utils::mask_email(e)),
            name: user.name,
            role: user.role,
            kyc_status: user.kyc_status,
            daily_limit: user.daily_limit,
            monthly_limit: user.monthly_limit,
            is_active: user.is_active,
            is_verified: user.is_verified,
            two_fa_enabled: user.two_fa_enabled,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl RefreshToken {
    pub fn new(user_id: Uuid, token_hash: String, expires_at: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            token_hash,
            expires_at,
            revoked: false,
            revoked_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.revoked && self.expires_at > Utc::now()
    }

    pub fn revoke(&mut self) {
        self.revoked = true;
        self.revoked_at = Some(Utc::now());
    }
}