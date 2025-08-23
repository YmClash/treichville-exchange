use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;

use crate::{
    domain::user::{User, UserRole, KycLevel, ChangeurProfile},
    shared::errors::AppError,
};

#[async_trait]
pub trait UserRepositoryTrait: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn find_by_phone(&self, phone: &str) -> Result<Option<User>, AppError>;
    async fn create(&self, user: User) -> Result<User, AppError>;
    async fn update(&self, user: &User) -> Result<User, AppError>;
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
    async fn get_changeur_profile(&self, user_id: Uuid) -> Result<Option<ChangeurProfile>, AppError>;
    async fn update_changeur_profile(&self, profile: &ChangeurProfile) -> Result<ChangeurProfile, AppError>;
}

pub struct UserRepository {
    db: Pool<Postgres>,
}

impl UserRepository {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserRepositoryTrait for UserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        Ok(user)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(&self.db)
        .await?;

        Ok(user)
    }

    async fn find_by_phone(&self, phone: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE phone = $1"
        )
        .bind(phone)
        .fetch_optional(&self.db)
        .await?;

        Ok(user)
    }

    async fn create(&self, user: User) -> Result<User, AppError> {
        let saved_user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (
                id, email, phone, password_hash, name,
                role, is_active, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.phone)
        .bind(&user.password_hash)
        .bind(&user.name)
        .bind(user.role.clone())
        .bind(user.is_active)
        .bind(user.created_at)
        .bind(user.updated_at)
        .fetch_one(&self.db)
        .await?;

        Ok(saved_user)
    }

    async fn update(&self, user: &User) -> Result<User, AppError> {
        let updated_user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users SET
                email = $2,
                phone = $3,
                name = $4,
                is_active = $5,
                kyc_status = $6,
                two_fa_enabled = $7,
                two_fa_secret = $8,
                updated_at = $9
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.phone)
        .bind(&user.name)
        .bind(user.is_active)
        .bind(user.kyc_status.clone())
        .bind(user.two_fa_enabled)
        .bind(&user.two_fa_secret)
        .bind(Utc::now())
        .fetch_one(&self.db)
        .await?;

        Ok(updated_user)
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET is_active = false WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    async fn get_changeur_profile(&self, user_id: Uuid) -> Result<Option<ChangeurProfile>, AppError> {
        let row = sqlx::query(
            "SELECT * FROM changeur_profiles WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some(row) => {
                let profile = build_changeur_profile_from_row(&row)?;
                Ok(Some(profile))
            },
            None => Ok(None)
        }
    }

    async fn update_changeur_profile(&self, profile: &ChangeurProfile) -> Result<ChangeurProfile, AppError> {
        let updated_profile = sqlx::query_as::<_, ChangeurProfile>(
            r#"
            UPDATE changeur_profiles SET
                business_name = $2,
                business_address = $3,
                rating = $4,
                total_transactions = $5,
                total_volume = $6,
                is_verified = $7,
                verified_at = $8,
                updated_at = $9
            WHERE user_id = $1
            RETURNING *
            "#
        )
        .bind(profile.user_id)
        .bind(&profile.business_name)
        .bind(&profile.business_address)
        .bind(profile.rating)
        .bind(profile.total_transactions as i64)
        .bind(profile.total_volume)
        .bind(profile.is_verified)
        .bind(profile.verified_at)
        .bind(Utc::now())
        .fetch_one(&self.db)
        .await?;

        Ok(updated_profile)
    }
}

// Helper function to build ChangeurProfile from database row
fn build_changeur_profile_from_row(row: &sqlx::postgres::PgRow) -> Result<ChangeurProfile, sqlx::Error> {
    use rust_decimal::Decimal;
    use chrono::{DateTime, Utc};
    
    Ok(ChangeurProfile {
        id: row.try_get("id")?,
        user_id: row.try_get("user_id")?,
        business_name: row.try_get("business_name")?,
        business_address: row.try_get("business_address")?,
        business_phone: row.try_get("business_phone")?,
        license_number: row.try_get("license_number")?,
        rating: row.try_get("rating")?,
        total_transactions: row.try_get("total_transactions")?,
        total_volume: row.try_get("total_volume")?,
        commission_rate: row.try_get("commission_rate")?,
        available_currencies: row.try_get("available_currencies").unwrap_or_else(|_| vec![]),
        operating_hours: row.try_get("operating_hours")?,
        location_latitude: row.try_get("location_latitude")?,
        location_longitude: row.try_get("location_longitude")?,
        is_verified: row.try_get("is_verified")?,
        verified_at: row.try_get("verified_at")?,
        metadata: row.try_get("metadata")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}