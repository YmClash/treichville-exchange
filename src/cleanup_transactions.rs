use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Connect to database
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/treichville_exchange".to_string());
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    println!("Connected to database");
    
    // Count bad transactions
    let count_result = sqlx::query!(
        "SELECT COUNT(*) as count FROM transactions WHERE amount_to > 100000"
    )
    .fetch_one(&pool)
    .await?;
    
    println!("Found {} transactions with incorrect amounts", count_result.count.unwrap_or(0));
    
    // Delete old pending transactions with incorrect amounts
    let deleted = sqlx::query!(
        "DELETE FROM transactions 
         WHERE status = 'pending' 
           AND amount_to > 100000 
           AND created_at < NOW() - INTERVAL '30 minutes'"
    )
    .execute(&pool)
    .await?;
    
    println!("Deleted {} old transactions with incorrect amounts", deleted.rows_affected());
    
    // Delete expired transactions
    let expired = sqlx::query!(
        "DELETE FROM transactions 
         WHERE status = 'pending' 
           AND expires_at IS NOT NULL
           AND expires_at < NOW() - INTERVAL '30 minutes'"
    )
    .execute(&pool)
    .await?;
    
    println!("Deleted {} expired transactions", expired.rows_affected());
    
    // Show summary
    let summary = sqlx::query!(
        "SELECT status, COUNT(*) as count 
         FROM transactions 
         GROUP BY status 
         ORDER BY status"
    )
    .fetch_all(&pool)
    .await?;
    
    println!("\nTransaction summary:");
    for row in summary {
        println!("  {}: {}", row.status, row.count.unwrap_or(0));
    }
    
    Ok(())
}