use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub app: AppConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub environment: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub acquire_timeout: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_expiry: i64,  // en secondes
    pub refresh_token_expiry: i64,  // en secondes
    pub issuer: String,
    pub audience: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let server = ServerConfig {

            host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000),
        };

        let database = DatabaseConfig {
            url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:password@localhost/treichville_exchange".to_string()),
            max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "25".to_string())
                .parse()
                .unwrap_or(25),
            acquire_timeout: 30,
        };

        let redis = RedisConfig {
            url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1/".to_string()),
        };

        let jwt = JwtConfig {
            secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "super-secret-key-change-in-production-min-32-chars".to_string()),
            access_token_expiry: std::env::var("JWT_ACCESS_TOKEN_EXPIRY")
                .unwrap_or_else(|_| "900".to_string())
                .parse::<i64>()
                .unwrap_or(900),
            refresh_token_expiry: std::env::var("JWT_REFRESH_TOKEN_EXPIRY")
                .unwrap_or_else(|_| "604800".to_string())
                .parse::<i64>()
                .unwrap_or(604800),
            issuer: std::env::var("JWT_ISSUER")
                .unwrap_or_else(|_| "treichville-exchange".to_string()),
            audience: std::env::var("JWT_AUDIENCE")
                .unwrap_or_else(|_| "treichville-exchange-api".to_string()),
        };

        let environment = std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string());

        let app = AppConfig {
            name: std::env::var("APP_NAME")
                .unwrap_or_else(|_| "Treichville Exchange".to_string()),
            url: std::env::var("APP_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        };

        Ok(Config {
            app,
            server,
            database,
            redis,
            jwt,
            environment,
        })
    }
}

pub type Settings = Config;