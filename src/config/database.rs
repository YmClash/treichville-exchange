use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::time::Duration;
use crate::config::settings::DatabaseSettings;

pub struct DatabaseConfig {
    pub pool: Pool<Postgres>,
}

impl DatabaseConfig {
    pub async fn new(settings: &DatabaseSettings) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(settings.max_connections)
            .min_connections(settings.min_connections)
            .acquire_timeout(Duration::from_secs(settings.connect_timeout))
            .idle_timeout(Duration::from_secs(settings.idle_timeout))
            .test_before_acquire(true)
            .connect(&settings.url)
            .await?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await?;

        Ok(Self { pool })
    }

    pub fn get_pool(&self) -> &Pool<Postgres> {
        &self.pool
    }
}