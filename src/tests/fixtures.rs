use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use crate::domain::{
    user::{User, UserRole, KycLevel},
    rate::ExchangeRate,
    transaction::{Transaction, TransactionStatus, TransactionType},
};

// Test data fixtures
pub struct TestFixtures;

impl TestFixtures {
    pub fn create_test_user(role: UserRole) -> User {
        User {
            id: Uuid::new_v4(),
            email: format!("test.{}@example.com", Uuid::new_v4()),
            phone: "+2250708090807".to_string(),
            password_hash: "hashed_password".to_string(),
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            role,
            kyc_level: KycLevel::Level1,
            is_active: true,
            email_verified: true,
            phone_verified: true,
            two_factor_enabled: false,
            two_factor_secret: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: Some(Utc::now()),
            failed_login_attempts: 0,
            locked_until: None,
            metadata: serde_json::json!({}),
        }
    }

    pub fn create_test_changeur() -> User {
        let mut user = Self::create_test_user(UserRole::Changeur);
        user.email = format!("changeur.{}@example.com", Uuid::new_v4());
        user
    }

    pub fn create_test_client() -> User {
        let mut user = Self::create_test_user(UserRole::Client);
        user.email = format!("client.{}@example.com", Uuid::new_v4());
        user
    }

    pub fn create_test_rate(changeur_id: Uuid) -> ExchangeRate {
        ExchangeRate {
            id: Uuid::new_v4(),
            changeur_id,
            from_currency: "EUR".to_string(),
            to_currency: "XOF".to_string(),
            buy_rate: Decimal::from(650),
            sell_rate: Decimal::from(660),
            mid_rate: Decimal::from(655),
            spread: Decimal::from(10),
            min_amount: Decimal::from(10),
            max_amount: Decimal::from(10000),
            available_amount: Some(Decimal::from(50000)),
            is_active: true,
            last_update_source: "manual".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn create_test_transaction(
        client_id: Uuid,
        changeur_id: Uuid,
    ) -> Transaction {
        Transaction {
            id: Uuid::new_v4(),
            client_id,
            changeur_id: Some(changeur_id),
            transaction_type: TransactionType::BuyCurrency,
            from_currency: "XOF".to_string(),
            to_currency: "EUR".to_string(),
            amount: Decimal::from(65000),
            rate: Decimal::from(650),
            fee: Decimal::from(100),
            total_amount: Decimal::from(65100),
            status: TransactionStatus::Pending,
            payment_method: "cash".to_string(),
            payment_reference: None,
            confirmation_code: Some(format!("{:06}", rand::random::<u32>() % 1000000)),
            expires_at: Some(Utc::now() + chrono::Duration::minutes(15)),
            completed_at: None,
            cancelled_at: None,
            cancellation_reason: None,
            idempotency_key: Uuid::new_v4().to_string(),
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn create_test_jwt_secret() -> String {
        "test_jwt_secret_key_that_is_at_least_32_characters_long".to_string()
    }

    pub fn create_test_database_url() -> String {
        std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/treichville_test".to_string())
    }

    pub fn create_test_redis_url() -> String {
        std::env::var("TEST_REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379/1".to_string())
    }
}

// Test helpers
pub mod helpers {
    use super::*;
    use sqlx::{PgPool, postgres::PgPoolOptions};
    use redis::aio::ConnectionManager;

    pub async fn setup_test_db() -> PgPool {
        let database_url = TestFixtures::create_test_database_url();
        
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        pool
    }

    pub async fn cleanup_test_db(pool: &PgPool) {
        // Clean up test data
        let tables = vec![
            "transaction_events",
            "transactions",
            "wallet_transactions",
            "wallets",
            "exchange_rates_history",
            "exchange_rates",
            "changeur_profiles",
            "refresh_tokens",
            "users",
        ];

        for table in tables {
            let query = format!("TRUNCATE TABLE {} CASCADE", table);
            sqlx::query(&query)
                .execute(pool)
                .await
                .expect(&format!("Failed to truncate {}", table));
        }
    }

    pub async fn setup_test_redis() -> ConnectionManager {
        let redis_url = TestFixtures::create_test_redis_url();
        let client = redis::Client::open(redis_url)
            .expect("Failed to create Redis client");
        
        ConnectionManager::new(client)
            .await
            .expect("Failed to connect to Redis")
    }

    pub async fn cleanup_test_redis(conn: &mut ConnectionManager) {
        use redis::AsyncCommands;
        let _: () = conn.flushdb()
            .await
            .expect("Failed to flush Redis test database");
    }
}

// Assertion helpers
pub mod assertions {
    use super::*;

    pub fn assert_user_equal(expected: &User, actual: &User) {
        assert_eq!(expected.email, actual.email);
        assert_eq!(expected.phone, actual.phone);
        assert_eq!(expected.first_name, actual.first_name);
        assert_eq!(expected.last_name, actual.last_name);
        assert_eq!(expected.role, actual.role);
        assert_eq!(expected.is_active, actual.is_active);
    }

    pub fn assert_rate_equal(expected: &ExchangeRate, actual: &ExchangeRate) {
        assert_eq!(expected.from_currency, actual.from_currency);
        assert_eq!(expected.to_currency, actual.to_currency);
        assert_eq!(expected.buy_rate, actual.buy_rate);
        assert_eq!(expected.sell_rate, actual.sell_rate);
        assert_eq!(expected.changeur_id, actual.changeur_id);
    }

    pub fn assert_transaction_equal(expected: &Transaction, actual: &Transaction) {
        assert_eq!(expected.client_id, actual.client_id);
        assert_eq!(expected.changeur_id, actual.changeur_id);
        assert_eq!(expected.transaction_type, actual.transaction_type);
        assert_eq!(expected.amount, actual.amount);
        assert_eq!(expected.status, actual.status);
    }

    pub fn assert_decimal_close(expected: Decimal, actual: Decimal, tolerance: Decimal) {
        let diff = (expected - actual).abs();
        assert!(
            diff <= tolerance,
            "Expected {} ± {}, but got {}",
            expected,
            tolerance,
            actual
        );
    }
}