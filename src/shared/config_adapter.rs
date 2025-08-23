// Adapter module to bridge between old Config and new Settings structure
use crate::config::settings::Settings;
use super::config::{Config, AppConfig, ServerConfig, DatabaseConfig, RedisConfig, JwtConfig};

impl From<Settings> for Config {
    fn from(settings: Settings) -> Self {
        Config {
            app: AppConfig {
                name: settings.app.name.clone(),
                url: format!("http://{}:{}", settings.app.host, settings.app.port),
            },
            server: ServerConfig {
                host: settings.app.host.clone(),
                port: settings.app.port,
            },
            database: DatabaseConfig {
                url: settings.database.url.clone(),
                max_connections: settings.database.max_connections,
                acquire_timeout: settings.database.acquire_timeout,
            },
            redis: RedisConfig {
                url: settings.redis.url.clone(),
            },
            jwt: JwtConfig {
                secret: settings.jwt.secret.clone(),
                access_token_expiry: settings.jwt.access_token_expiry,
                refresh_token_expiry: settings.jwt.refresh_token_expiry,
                issuer: settings.jwt.issuer.clone(),
                audience: "treichville-exchange-api".to_string(),
            },
            environment: settings.app.env.clone(),
        }
    }
}

// Helper function to convert Settings to Config where needed
pub fn settings_to_config(settings: &Settings) -> Config {
    Config::from(settings.clone())
}