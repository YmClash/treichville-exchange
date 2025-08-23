use redis::{aio::ConnectionManager, AsyncCommands, RedisError};
use std::sync::Arc;
use tracing::{error, warn};
use crate::infrastructure::redis_pool::RedisPool;

/// Wrapper around RedisPool that provides resilient Redis operations
pub struct RedisWrapper {
    pool: Arc<RedisPool>,
}

impl RedisWrapper {
    pub fn new(pool: Arc<RedisPool>) -> Self {
        Self { pool }
    }
    
    /// Execute a Redis GET command with automatic retry
    pub async fn get<T: redis::FromRedisValue>(&self, key: &str) -> Result<Option<T>, RedisError> {
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 3;
        
        while attempts < MAX_ATTEMPTS {
            attempts += 1;
            
            match self.pool.get_connection().await {
                Ok(mut conn) => {
                    match conn.get(key).await {
                        Ok(value) => return Ok(value),
                        Err(e) if Self::is_connection_error(&e) && attempts < MAX_ATTEMPTS => {
                            warn!("Redis GET failed (attempt {}/{}): {}", attempts, MAX_ATTEMPTS, e);
                            tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts as u64)).await;
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }
                Err(e) if attempts < MAX_ATTEMPTS => {
                    warn!("Failed to get Redis connection (attempt {}/{}): {}", attempts, MAX_ATTEMPTS, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts as u64)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        
        Err(RedisError::from((redis::ErrorKind::IoError, "Max retry attempts exceeded")))
    }
    
    /// Execute a Redis SET command with automatic retry
    pub async fn set<T: redis::ToRedisArgs + Clone + Send + Sync>(&self, key: &str, value: T) -> Result<(), RedisError> {
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 3;
        
        while attempts < MAX_ATTEMPTS {
            attempts += 1;
            
            match self.pool.get_connection().await {
                Ok(mut conn) => {
                    match conn.set(key, value.clone()).await {
                        Ok(()) => return Ok(()),
                        Err(e) if Self::is_connection_error(&e) && attempts < MAX_ATTEMPTS => {
                            warn!("Redis SET failed (attempt {}/{}): {}", attempts, MAX_ATTEMPTS, e);
                            tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts as u64)).await;
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }
                Err(e) if attempts < MAX_ATTEMPTS => {
                    warn!("Failed to get Redis connection (attempt {}/{}): {}", attempts, MAX_ATTEMPTS, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts as u64)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        
        Err(RedisError::from((redis::ErrorKind::IoError, "Max retry attempts exceeded")))
    }
    
    /// Execute a Redis SETEX command with automatic retry
    pub async fn setex<T: redis::ToRedisArgs + Clone + Send + Sync>(&self, key: &str, value: T, seconds: u64) -> Result<(), RedisError> {
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 3;
        
        while attempts < MAX_ATTEMPTS {
            attempts += 1;
            
            match self.pool.get_connection().await {
                Ok(mut conn) => {
                    match conn.set_ex(key, value.clone(), seconds).await {
                        Ok(()) => return Ok(()),
                        Err(e) if Self::is_connection_error(&e) && attempts < MAX_ATTEMPTS => {
                            warn!("Redis SETEX failed (attempt {}/{}): {}", attempts, MAX_ATTEMPTS, e);
                            tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts as u64)).await;
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }
                Err(e) if attempts < MAX_ATTEMPTS => {
                    warn!("Failed to get Redis connection (attempt {}/{}): {}", attempts, MAX_ATTEMPTS, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts as u64)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        
        Err(RedisError::from((redis::ErrorKind::IoError, "Max retry attempts exceeded")))
    }
    
    /// Execute a Redis DEL command with automatic retry
    pub async fn del(&self, key: &str) -> Result<(), RedisError> {
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 3;
        
        while attempts < MAX_ATTEMPTS {
            attempts += 1;
            
            match self.pool.get_connection().await {
                Ok(mut conn) => {
                    match conn.del(key).await {
                        Ok(()) => return Ok(()),
                        Err(e) if Self::is_connection_error(&e) && attempts < MAX_ATTEMPTS => {
                            warn!("Redis DEL failed (attempt {}/{}): {}", attempts, MAX_ATTEMPTS, e);
                            tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts as u64)).await;
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }
                Err(e) if attempts < MAX_ATTEMPTS => {
                    warn!("Failed to get Redis connection (attempt {}/{}): {}", attempts, MAX_ATTEMPTS, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts as u64)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        
        Err(RedisError::from((redis::ErrorKind::IoError, "Max retry attempts exceeded")))
    }
    
    /// Check if an error is a connection error that should trigger a retry
    fn is_connection_error(error: &RedisError) -> bool {
        match error.kind() {
            redis::ErrorKind::IoError => true,
            redis::ErrorKind::BusyLoadingError => true,
            redis::ErrorKind::TryAgain => true,
            _ => {
                let error_str = error.to_string().to_lowercase();
                error_str.contains("broken pipe") || 
                error_str.contains("connection refused") ||
                error_str.contains("connection reset") ||
                error_str.contains("timed out")
            }
        }
    }
    
    /// Get a raw connection for complex operations
    pub async fn get_connection(&self) -> Result<ConnectionManager, RedisError> {
        self.pool.get_connection().await
    }
}