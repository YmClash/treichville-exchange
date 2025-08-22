use redis::aio::ConnectionManager;
use redis::{Client, RedisError};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Redis connection pool with automatic reconnection
pub struct RedisPool {
    client: Client,
    conn: Arc<RwLock<Option<ConnectionManager>>>,
    url: String,
}

impl RedisPool {
    pub async fn new(url: String) -> Result<Self, RedisError> {
        let client = Client::open(url.clone())?;
        let conn_manager = ConnectionManager::new(client.clone()).await?;
        
        info!("Redis connection pool initialized");
        
        Ok(Self {
            client,
            conn: Arc::new(RwLock::new(Some(conn_manager))),
            url,
        })
    }
    
    /// Get a connection, reconnecting if necessary
    pub async fn get_connection(&self) -> Result<ConnectionManager, RedisError> {
        let conn_guard = self.conn.read().await;
        
        if let Some(conn) = conn_guard.as_ref() {
            // Test if connection is still alive
            let mut test_conn = conn.clone();
            match redis::cmd("PING").query_async::<_, String>(&mut test_conn).await {
                Ok(_) => return Ok(conn.clone()),
                Err(e) => {
                    warn!("Redis connection test failed, reconnecting: {}", e);
                    drop(conn_guard); // Release read lock
                }
            }
        } else {
            drop(conn_guard); // Release read lock
        }
        
        // Need to reconnect
        self.reconnect().await
    }
    
    async fn reconnect(&self) -> Result<ConnectionManager, RedisError> {
        let mut conn_guard = self.conn.write().await;
        
        // Double check in case another thread already reconnected
        if let Some(conn) = conn_guard.as_ref() {
            let mut test_conn = conn.clone();
            if redis::cmd("PING").query_async::<_, String>(&mut test_conn).await.is_ok() {
                return Ok(conn.clone());
            }
        }
        
        // Attempt reconnection with retries
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 3;
        
        loop {
            attempts += 1;
            match ConnectionManager::new(self.client.clone()).await {
                Ok(new_conn) => {
                    info!("Redis reconnection successful after {} attempts", attempts);
                    *conn_guard = Some(new_conn.clone());
                    return Ok(new_conn);
                }
                Err(e) if attempts >= MAX_ATTEMPTS => {
                    error!("Failed to reconnect to Redis after {} attempts: {}", attempts, e);
                    *conn_guard = None;
                    return Err(e);
                }
                Err(e) => {
                    warn!("Redis reconnection attempt {} failed: {}, retrying...", attempts, e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
    
    /// Execute a Redis command with automatic retry on connection failure
    pub async fn execute<T, F, Fut>(&self, f: F) -> Result<T, RedisError>
    where
        F: Fn(ConnectionManager) -> Fut,
        Fut: std::future::Future<Output = Result<T, RedisError>>,
    {
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 2;
        
        loop {
            attempts += 1;
            let conn = self.get_connection().await?;
            
            match f(conn).await {
                Ok(result) => return Ok(result),
                Err(e) if attempts >= MAX_ATTEMPTS => {
                    error!("Redis command failed after {} attempts: {}", attempts, e);
                    return Err(e);
                }
                Err(e) if Self::is_connection_error(&e) => {
                    warn!("Redis connection error, retrying: {}", e);
                    // Force reconnection on next attempt
                    let mut conn_guard = self.conn.write().await;
                    *conn_guard = None;
                }
                Err(e) => return Err(e), // Non-connection error, don't retry
            }
        }
    }
    
    fn is_connection_error(error: &RedisError) -> bool {
        let error_str = error.to_string().to_lowercase();
        error_str.contains("broken pipe") ||
        error_str.contains("connection refused") ||
        error_str.contains("connection reset") ||
        error_str.contains("eof") ||
        error_str.contains("i/o error")
    }
}

impl Clone for RedisPool {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            conn: self.conn.clone(),
            url: self.url.clone(),
        }
    }
}