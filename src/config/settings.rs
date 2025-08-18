use serde::Deserialize;
use config::{Config, ConfigError, File, Environment};
use std::env;
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub app: AppConfig,
    pub database: DatabaseSettings,
    pub redis: RedisSettings,
    pub jwt: JwtConfig,
    pub security: SecurityConfig,
    pub binance: BinanceConfig,
    pub mobile_money: MobileMoneyConfig,
    pub transaction: TransactionConfig,
    pub kyc: KycConfig,
    pub monitoring: MonitoringConfig,
    pub websocket: WebSocketConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub name: String,
    pub env: String,
    pub host: String,
    pub port: u16,
    pub log_level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSettings {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: u64,
    pub idle_timeout: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisSettings {
    pub url: String,
    pub pool_size: u32,
    pub ttl_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_expiry: i64,
    pub refresh_token_expiry: i64,
    pub issuer: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    pub argon2_salt: String,
    pub rate_limit_per_second: u64,
    pub rate_limit_burst: u32,
    pub cors_allowed_origins: Vec<String>,
    pub enable_2fa: bool,
    pub enable_rate_limiting: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BinanceConfig {
    pub api_key: String,
    pub secret_key: String,
    pub base_url: String,
    pub ws_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MobileMoneyConfig {
    pub orange_money: OrangeMoneyConfig,
    pub wave: WaveConfig,
    pub mtn_momo: MtnMomoConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OrangeMoneyConfig {
    pub api_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub merchant_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WaveConfig {
    pub api_url: String,
    pub api_key: String,
    pub webhook_secret: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MtnMomoConfig {
    pub api_url: String,
    pub subscription_key: String,
    pub callback_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TransactionConfig {
    pub default_timeout_minutes: u32,
    pub max_amount_xof: rust_decimal::Decimal,
    pub min_amount_xof: rust_decimal::Decimal,
    pub default_fee_percentage: rust_decimal::Decimal,
    pub idempotency_key_ttl: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct KycConfig {
    pub level_0_daily_limit: rust_decimal::Decimal,
    pub level_0_monthly_limit: rust_decimal::Decimal,
    pub level_1_daily_limit: rust_decimal::Decimal,
    pub level_1_monthly_limit: rust_decimal::Decimal,
    pub level_2_daily_limit: rust_decimal::Decimal,
    pub level_2_monthly_limit: rust_decimal::Decimal,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MonitoringConfig {
    pub otel_endpoint: String,
    pub service_name: String,
    pub jaeger_agent_host: String,
    pub jaeger_agent_port: u16,
    pub enable_audit_log: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebSocketConfig {
    pub max_connections: usize,
    pub ping_interval: u64,
    pub max_message_size: usize,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("APP_ENV").unwrap_or_else(|_| "development".into());
        
        let s = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            .add_source(File::with_name("config/local").required(false))
            .add_source(Environment::with_prefix("APP").separator("_"))
            .build()?;
        
        s.try_deserialize()
    }

    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();
        
        Ok(Settings {
            app: AppConfig {
                name: env::var("APP_NAME").unwrap_or_else(|_| "treichville_exchange".to_string()),
                env: env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
                host: env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("APP_PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .expect("APP_PORT must be a number"),
                log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            },
            database: DatabaseSettings {
                url: env::var("DATABASE_URL")
                    .expect("DATABASE_URL must be set"),
                max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "100".to_string())
                    .parse()
                    .expect("DATABASE_MAX_CONNECTIONS must be a number"),
                min_connections: env::var("DATABASE_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .expect("DATABASE_MIN_CONNECTIONS must be a number"),
                connect_timeout: 30,
                idle_timeout: 600,
            },
            redis: RedisSettings {
                url: env::var("REDIS_URL")
                    .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
                pool_size: env::var("REDIS_POOL_SIZE")
                    .unwrap_or_else(|_| "50".to_string())
                    .parse()
                    .expect("REDIS_POOL_SIZE must be a number"),
                ttl_seconds: 3600,
            },
            jwt: JwtConfig {
                secret: env::var("JWT_SECRET")
                    .expect("JWT_SECRET must be set"),
                access_token_expiry: env::var("JWT_ACCESS_TOKEN_EXPIRY")
                    .unwrap_or_else(|_| "900".to_string())
                    .parse()
                    .expect("JWT_ACCESS_TOKEN_EXPIRY must be a number"),
                refresh_token_expiry: env::var("JWT_REFRESH_TOKEN_EXPIRY")
                    .unwrap_or_else(|_| "604800".to_string())
                    .parse()
                    .expect("JWT_REFRESH_TOKEN_EXPIRY must be a number"),
                issuer: env::var("JWT_ISSUER")
                    .unwrap_or_else(|_| "treichville-exchange".to_string()),
            },
            security: SecurityConfig {
                argon2_salt: env::var("ARGON2_SALT")
                    .expect("ARGON2_SALT must be set"),
                rate_limit_per_second: env::var("RATE_LIMIT_PER_SECOND")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .expect("RATE_LIMIT_PER_SECOND must be a number"),
                rate_limit_burst: env::var("RATE_LIMIT_BURST")
                    .unwrap_or_else(|_| "100".to_string())
                    .parse()
                    .expect("RATE_LIMIT_BURST must be a number"),
                cors_allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                enable_2fa: env::var("ENABLE_2FA")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                enable_rate_limiting: env::var("ENABLE_RATE_LIMITING")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
            },
            binance: BinanceConfig {
                api_key: env::var("BINANCE_API_KEY").unwrap_or_default(),
                secret_key: env::var("BINANCE_SECRET_KEY").unwrap_or_default(),
                base_url: env::var("BINANCE_BASE_URL")
                    .unwrap_or_else(|_| "https://api.binance.com".to_string()),
                ws_url: env::var("BINANCE_WS_URL")
                    .unwrap_or_else(|_| "wss://stream.binance.com:9443/ws".to_string()),
            },
            mobile_money: MobileMoneyConfig {
                orange_money: OrangeMoneyConfig {
                    api_url: env::var("ORANGE_MONEY_API_URL")
                        .unwrap_or_else(|_| "https://api.orange.com/orange-money-webpay/dev/v1".to_string()),
                    client_id: env::var("ORANGE_MONEY_CLIENT_ID").unwrap_or_default(),
                    client_secret: env::var("ORANGE_MONEY_CLIENT_SECRET").unwrap_or_default(),
                    merchant_key: env::var("ORANGE_MONEY_MERCHANT_KEY").unwrap_or_default(),
                },
                wave: WaveConfig {
                    api_url: env::var("WAVE_API_URL")
                        .unwrap_or_else(|_| "https://api.wave.com/v1".to_string()),
                    api_key: env::var("WAVE_API_KEY").unwrap_or_default(),
                    webhook_secret: env::var("WAVE_WEBHOOK_SECRET").unwrap_or_default(),
                },
                mtn_momo: MtnMomoConfig {
                    api_url: env::var("MTN_MOMO_API_URL")
                        .unwrap_or_else(|_| "https://proxy.momoapi.mtn.com".to_string()),
                    subscription_key: env::var("MTN_MOMO_SUBSCRIPTION_KEY").unwrap_or_default(),
                    callback_url: env::var("MTN_MOMO_CALLBACK_URL").unwrap_or_default(),
                },
            },
            transaction: TransactionConfig {
                default_timeout_minutes: env::var("DEFAULT_TRANSACTION_TIMEOUT_MINUTES")
                    .unwrap_or_else(|_| "15".to_string())
                    .parse()
                    .expect("DEFAULT_TRANSACTION_TIMEOUT_MINUTES must be a number"),
                max_amount_xof: env::var("MAX_TRANSACTION_AMOUNT_XOF")
                    .unwrap_or_else(|_| "10000000".to_string())
                    .parse()
                    .expect("MAX_TRANSACTION_AMOUNT_XOF must be a valid decimal"),
                min_amount_xof: env::var("MIN_TRANSACTION_AMOUNT_XOF")
                    .unwrap_or_else(|_| "1000".to_string())
                    .parse()
                    .expect("MIN_TRANSACTION_AMOUNT_XOF must be a valid decimal"),
                default_fee_percentage: env::var("DEFAULT_FEE_PERCENTAGE")
                    .unwrap_or_else(|_| "0.5".to_string())
                    .parse()
                    .expect("DEFAULT_FEE_PERCENTAGE must be a valid decimal"),
                idempotency_key_ttl: env::var("IDEMPOTENCY_KEY_TTL")
                    .unwrap_or_else(|_| "86400".to_string())
                    .parse()
                    .expect("IDEMPOTENCY_KEY_TTL must be a number"),
            },
            kyc: KycConfig {
                level_0_daily_limit: env::var("KYC_LEVEL_0_DAILY_LIMIT")
                    .unwrap_or_else(|_| "50000".to_string())
                    .parse()
                    .expect("KYC_LEVEL_0_DAILY_LIMIT must be a valid decimal"),
                level_0_monthly_limit: env::var("KYC_LEVEL_0_MONTHLY_LIMIT")
                    .unwrap_or_else(|_| "500000".to_string())
                    .parse()
                    .expect("KYC_LEVEL_0_MONTHLY_LIMIT must be a valid decimal"),
                level_1_daily_limit: env::var("KYC_LEVEL_1_DAILY_LIMIT")
                    .unwrap_or_else(|_| "500000".to_string())
                    .parse()
                    .expect("KYC_LEVEL_1_DAILY_LIMIT must be a valid decimal"),
                level_1_monthly_limit: env::var("KYC_LEVEL_1_MONTHLY_LIMIT")
                    .unwrap_or_else(|_| "5000000".to_string())
                    .parse()
                    .expect("KYC_LEVEL_1_MONTHLY_LIMIT must be a valid decimal"),
                level_2_daily_limit: env::var("KYC_LEVEL_2_DAILY_LIMIT")
                    .unwrap_or_else(|_| "5000000".to_string())
                    .parse()
                    .expect("KYC_LEVEL_2_DAILY_LIMIT must be a valid decimal"),
                level_2_monthly_limit: env::var("KYC_LEVEL_2_MONTHLY_LIMIT")
                    .unwrap_or_else(|_| "50000000".to_string())
                    .parse()
                    .expect("KYC_LEVEL_2_MONTHLY_LIMIT must be a valid decimal"),
            },
            monitoring: MonitoringConfig {
                otel_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                    .unwrap_or_else(|_| "http://localhost:4317".to_string()),
                service_name: env::var("OTEL_SERVICE_NAME")
                    .unwrap_or_else(|_| "treichville-exchange".to_string()),
                jaeger_agent_host: env::var("JAEGER_AGENT_HOST")
                    .unwrap_or_else(|_| "localhost".to_string()),
                jaeger_agent_port: env::var("JAEGER_AGENT_PORT")
                    .unwrap_or_else(|_| "6831".to_string())
                    .parse()
                    .expect("JAEGER_AGENT_PORT must be a number"),
                enable_audit_log: env::var("ENABLE_AUDIT_LOG")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
            },
            websocket: WebSocketConfig {
                max_connections: env::var("WS_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10000".to_string())
                    .parse()
                    .expect("WS_MAX_CONNECTIONS must be a number"),
                ping_interval: env::var("WS_PING_INTERVAL")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .expect("WS_PING_INTERVAL must be a number"),
                max_message_size: env::var("WS_MAX_MESSAGE_SIZE")
                    .unwrap_or_else(|_| "65536".to_string())
                    .parse()
                    .expect("WS_MAX_MESSAGE_SIZE must be a number"),
            },
        })
    }

    pub fn get_database_url(&self) -> &str {
        &self.database.url
    }

    pub fn get_redis_url(&self) -> &str {
        &self.redis.url
    }

    pub fn get_bind_address(&self) -> String {
        format!("{}:{}", self.app.host, self.app.port)
    }
}