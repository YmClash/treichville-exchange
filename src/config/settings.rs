use serde::Deserialize;
use config::{Config, ConfigError, File, Environment};
use std::env;
use std::time::Duration;
use tracing::{error, warn};

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
    pub acquire_timeout: u64,
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

// Helper function to parse environment variable with default
fn parse_env_var<T: std::str::FromStr + std::fmt::Debug>(key: &str, default: T) -> Result<T, ConfigError> 
where 
    T::Err: std::fmt::Display
{
    env::var(key)
        .unwrap_or_else(|_| format!("{:?}", default))
        .parse::<T>()
        .map_err(|e| {
            let msg = format!("{} must be a valid value: {}", key, e);
            error!("{}", msg);
            ConfigError::Message(msg)
        })
}

// Helper function for required environment variables
fn require_env_var(key: &str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| {
        let msg = format!("{} environment variable is required but not set", key);
        error!("{}", msg);
        ConfigError::Message(msg)
    })
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
        
        use rust_decimal::Decimal;
        use std::str::FromStr;
        
        // Parse with proper error handling
        let app_port = env::var("APP_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .map_err(|e| {
                warn!("Invalid APP_PORT, using default 8080: {}", e);
                ConfigError::Message("APP_PORT must be a valid port number".into())
            })
            .unwrap_or(8080);
        
        let database_url = require_env_var("DATABASE_URL")?;
        let jwt_secret = require_env_var("JWT_SECRET")?;
        let argon2_salt = require_env_var("ARGON2_SALT")?;
        
        // Parse numeric values with defaults
        let database_max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "100".to_string())
            .parse()
            .unwrap_or_else(|e| {
                warn!("Invalid DATABASE_MAX_CONNECTIONS, using default 100: {}", e);
                100
            });
        
        let database_min_connections = env::var("DATABASE_MIN_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .unwrap_or_else(|e| {
                warn!("Invalid DATABASE_MIN_CONNECTIONS, using default 10: {}", e);
                10
            });
        
        let redis_pool_size = env::var("REDIS_POOL_SIZE")
            .unwrap_or_else(|_| "50".to_string())
            .parse()
            .unwrap_or_else(|e| {
                warn!("Invalid REDIS_POOL_SIZE, using default 50: {}", e);
                50
            });
        
        let jwt_access_token_expiry = env::var("JWT_ACCESS_TOKEN_EXPIRY")
            .unwrap_or_else(|_| "900".to_string())
            .parse()
            .unwrap_or_else(|e| {
                warn!("Invalid JWT_ACCESS_TOKEN_EXPIRY, using default 900: {}", e);
                900
            });
        
        let jwt_refresh_token_expiry = env::var("JWT_REFRESH_TOKEN_EXPIRY")
            .unwrap_or_else(|_| "604800".to_string())
            .parse()
            .unwrap_or_else(|e| {
                warn!("Invalid JWT_REFRESH_TOKEN_EXPIRY, using default 604800: {}", e);
                604800
            });
        
        let rate_limit_per_second = env::var("RATE_LIMIT_PER_SECOND")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .unwrap_or_else(|e| {
                warn!("Invalid RATE_LIMIT_PER_SECOND, using default 10: {}", e);
                10
            });
        
        let rate_limit_burst = env::var("RATE_LIMIT_BURST")
            .unwrap_or_else(|_| "100".to_string())
            .parse()
            .unwrap_or_else(|e| {
                warn!("Invalid RATE_LIMIT_BURST, using default 100: {}", e);
                100
            });
        
        // Parse decimal values with proper error handling
        let parse_decimal = |key: &str, default: &str| -> Decimal {
            env::var(key)
                .unwrap_or_else(|_| default.to_string())
                .parse()
                .unwrap_or_else(|e| {
                    warn!("Invalid {}, using default {}: {}", key, default, e);
                    Decimal::from_str(default).unwrap_or(Decimal::ZERO)
                })
        };
        
        Ok(Settings {
            app: AppConfig {
                name: env::var("APP_NAME").unwrap_or_else(|_| "treichville_exchange".to_string()),
                env: env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
                host: env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: app_port,
                log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            },
            database: DatabaseSettings {
                url: database_url,
                max_connections: database_max_connections,
                min_connections: database_min_connections,
                connect_timeout: 30,
                idle_timeout: 600,
                acquire_timeout: env::var("DATABASE_ACQUIRE_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
            },
            redis: RedisSettings {
                url: env::var("REDIS_URL")
                    .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
                pool_size: redis_pool_size,
                ttl_seconds: 3600,
            },
            jwt: JwtConfig {
                secret: jwt_secret,
                access_token_expiry: jwt_access_token_expiry,
                refresh_token_expiry: jwt_refresh_token_expiry,
                issuer: env::var("JWT_ISSUER")
                    .unwrap_or_else(|_| "treichville-exchange".to_string()),
            },
            security: SecurityConfig {
                argon2_salt,
                rate_limit_per_second,
                rate_limit_burst,
                cors_allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                enable_2fa: env::var("ENABLE_2FA")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
                enable_rate_limiting: env::var("ENABLE_RATE_LIMITING")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
            },
            binance: BinanceConfig {
                api_key: env::var("BINANCE_API_KEY")
                    .unwrap_or_else(|_| String::new()),
                secret_key: env::var("BINANCE_SECRET_KEY")
                    .unwrap_or_else(|_| String::new()),
                base_url: env::var("BINANCE_BASE_URL")
                    .unwrap_or_else(|_| "https://api.binance.com".to_string()),
                ws_url: env::var("BINANCE_WS_URL")
                    .unwrap_or_else(|_| "wss://stream.binance.com:9443".to_string()),
            },
            mobile_money: MobileMoneyConfig {
                orange_money: OrangeMoneyConfig {
                    api_url: env::var("ORANGE_MONEY_API_URL")
                        .unwrap_or_else(|_| "https://api.orange.com".to_string()),
                    client_id: env::var("ORANGE_MONEY_CLIENT_ID")
                        .unwrap_or_else(|_| String::new()),
                    client_secret: env::var("ORANGE_MONEY_CLIENT_SECRET")
                        .unwrap_or_else(|_| String::new()),
                    merchant_key: env::var("ORANGE_MONEY_MERCHANT_KEY")
                        .unwrap_or_else(|_| String::new()),
                },
                wave: WaveConfig {
                    api_url: env::var("WAVE_API_URL")
                        .unwrap_or_else(|_| "https://api.wave.com".to_string()),
                    api_key: env::var("WAVE_API_KEY")
                        .unwrap_or_else(|_| String::new()),
                    webhook_secret: env::var("WAVE_WEBHOOK_SECRET")
                        .unwrap_or_else(|_| String::new()),
                },
                mtn_momo: MtnMomoConfig {
                    api_url: env::var("MTN_MOMO_API_URL")
                        .unwrap_or_else(|_| "https://api.mtn.com".to_string()),
                    subscription_key: env::var("MTN_MOMO_SUBSCRIPTION_KEY")
                        .unwrap_or_else(|_| String::new()),
                    callback_url: env::var("MTN_MOMO_CALLBACK_URL")
                        .unwrap_or_else(|_| "http://localhost:8080/webhooks/mtn".to_string()),
                },
            },
            transaction: TransactionConfig {
                default_timeout_minutes: env::var("DEFAULT_TRANSACTION_TIMEOUT_MINUTES")
                    .unwrap_or_else(|_| "15".to_string())
                    .parse()
                    .unwrap_or(15),
                max_amount_xof: parse_decimal("MAX_TRANSACTION_AMOUNT_XOF", "10000000"),
                min_amount_xof: parse_decimal("MIN_TRANSACTION_AMOUNT_XOF", "1000"),
                default_fee_percentage: parse_decimal("DEFAULT_FEE_PERCENTAGE", "0.5"),
                idempotency_key_ttl: env::var("IDEMPOTENCY_KEY_TTL")
                    .unwrap_or_else(|_| "86400".to_string())
                    .parse()
                    .unwrap_or(86400),
            },
            kyc: KycConfig {
                level_0_daily_limit: parse_decimal("KYC_LEVEL_0_DAILY_LIMIT", "50000"),
                level_0_monthly_limit: parse_decimal("KYC_LEVEL_0_MONTHLY_LIMIT", "500000"),
                level_1_daily_limit: parse_decimal("KYC_LEVEL_1_DAILY_LIMIT", "500000"),
                level_1_monthly_limit: parse_decimal("KYC_LEVEL_1_MONTHLY_LIMIT", "5000000"),
                level_2_daily_limit: parse_decimal("KYC_LEVEL_2_DAILY_LIMIT", "5000000"),
                level_2_monthly_limit: parse_decimal("KYC_LEVEL_2_MONTHLY_LIMIT", "50000000"),
            },
            monitoring: MonitoringConfig {
                otel_endpoint: env::var("OTEL_ENDPOINT")
                    .unwrap_or_else(|_| "http://localhost:4317".to_string()),
                service_name: env::var("SERVICE_NAME")
                    .unwrap_or_else(|_| "treichville-exchange".to_string()),
                jaeger_agent_host: env::var("JAEGER_AGENT_HOST")
                    .unwrap_or_else(|_| "localhost".to_string()),
                jaeger_agent_port: env::var("JAEGER_AGENT_PORT")
                    .unwrap_or_else(|_| "6831".to_string())
                    .parse()
                    .unwrap_or(6831),
                enable_audit_log: env::var("ENABLE_AUDIT_LOG")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
            },
            websocket: WebSocketConfig {
                max_connections: env::var("WS_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10000".to_string())
                    .parse()
                    .unwrap_or(10000),
                ping_interval: env::var("WS_PING_INTERVAL")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
                max_message_size: env::var("WS_MAX_MESSAGE_SIZE")
                    .unwrap_or_else(|_| "65536".to_string())
                    .parse()
                    .unwrap_or(65536),
            },
        })
    }
}