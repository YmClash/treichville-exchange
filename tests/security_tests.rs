use std::sync::Arc;
use tokio::sync::Barrier;
use rust_decimal::Decimal;
use uuid::Uuid;
use sqlx::postgres::PgPoolOptions;

#[cfg(test)]
mod security_tests {
    use super::*;
    use treichville_exchange::application::exchange::{
        atomic_operations::AtomicWalletService,
        wallet_service::WalletService,
    };
    
    /// Test pour vérifier qu'il n'y a pas de double-spending même avec des requêtes parallèles
    #[tokio::test]
    async fn test_no_double_spending_with_concurrent_requests() {
        // Setup database connection
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/treichville_test".to_string());
        
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await
            .expect("Failed to connect to database");
        
        // Create wallet service
        let wallet_service = Arc::new(WalletService::new(pool.clone()));
        
        // Create test user and wallet
        let user_id = Uuid::new_v4();
        let initial_balance = Decimal::from(1000);
        
        // Setup initial wallet with balance
        let wallet = wallet_service
            .get_or_create_wallet(user_id, "XOF")
            .await
            .expect("Failed to create wallet");
        
        // Credit initial balance
        wallet_service
            .credit(user_id, "XOF", initial_balance, "initial-credit")
            .await
            .expect("Failed to credit initial balance");
        
        // Number of concurrent threads
        let num_threads = 100;
        let amount_per_thread = Decimal::from(20); // Total would be 2000, but we only have 1000
        
        // Barrier to synchronize all threads
        let barrier = Arc::new(Barrier::new(num_threads));
        let mut handles = vec![];
        
        for i in 0..num_threads {
            let wallet_service = wallet_service.clone();
            let barrier = barrier.clone();
            
            let handle = tokio::spawn(async move {
                // Wait for all threads to be ready
                barrier.wait().await;
                
                // Try to debit
                let result = wallet_service
                    .debit(user_id, "XOF", amount_per_thread, &format!("debit-{}", i))
                    .await;
                
                result.is_ok()
            });
            
            handles.push(handle);
        }
        
        // Collect results
        let mut successful_debits = 0;
        for handle in handles {
            if handle.await.expect("Thread panicked") {
                successful_debits += 1;
            }
        }
        
        // Check final balance
        let final_balance = wallet_service
            .get_balance(user_id, "XOF")
            .await
            .expect("Failed to get balance");
        
        // Verify that we never went negative
        assert!(final_balance.total_balance >= Decimal::ZERO, "Balance went negative!");
        
        // Verify that successful debits match the available balance
        let expected_successful = 50; // 1000 / 20 = 50
        assert_eq!(
            successful_debits, expected_successful,
            "Expected {} successful debits, got {}",
            expected_successful, successful_debits
        );
        
        // Verify final balance is correct
        let expected_balance = initial_balance - (Decimal::from(successful_debits) * amount_per_thread);
        assert_eq!(
            final_balance.total_balance, expected_balance,
            "Final balance mismatch"
        );
        
        println!("✅ No double-spending detected with {} concurrent requests", num_threads);
        println!("   Initial balance: {}", initial_balance);
        println!("   Successful debits: {}/{}", successful_debits, num_threads);
        println!("   Final balance: {}", final_balance.total_balance);
    }
    
    /// Test pour vérifier l'atomicité des transferts
    #[tokio::test]
    async fn test_atomic_transfer_rollback_on_failure() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/treichville_test".to_string());
        
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to connect to database");
        
        let atomic_service = AtomicWalletService::new(pool.clone());
        let wallet_service = WalletService::new(pool.clone());
        
        // Create two users
        let user1_id = Uuid::new_v4();
        let user2_id = Uuid::new_v4();
        
        // Setup wallets
        let wallet1 = wallet_service
            .get_or_create_wallet(user1_id, "XOF")
            .await
            .expect("Failed to create wallet1");
        
        let wallet2 = wallet_service
            .get_or_create_wallet(user2_id, "XOF")
            .await
            .expect("Failed to create wallet2");
        
        // Credit user1
        wallet_service
            .credit(user1_id, "XOF", Decimal::from(1000), "initial")
            .await
            .expect("Failed to credit");
        
        // Get initial balances
        let initial_balance1 = wallet_service
            .get_balance(user1_id, "XOF")
            .await
            .expect("Failed to get balance")
            .total_balance;
        
        let initial_balance2 = wallet_service
            .get_balance(user2_id, "XOF")
            .await
            .expect("Failed to get balance")
            .total_balance;
        
        // Try to transfer more than available (should fail atomically)
        let transfer_amount = Decimal::from(2000); // More than available
        let result = atomic_service
            .transfer_atomic(
                wallet1.id,
                wallet2.id,
                transfer_amount,
                "XOF",
                Uuid::new_v4(),
                "test-transfer-fail",
            )
            .await;
        
        // Transfer should fail
        assert!(result.is_err(), "Transfer should have failed");
        
        // Check that balances are unchanged (atomic rollback)
        let final_balance1 = wallet_service
            .get_balance(user1_id, "XOF")
            .await
            .expect("Failed to get balance")
            .total_balance;
        
        let final_balance2 = wallet_service
            .get_balance(user2_id, "XOF")
            .await
            .expect("Failed to get balance")
            .total_balance;
        
        assert_eq!(
            initial_balance1, final_balance1,
            "User1 balance should be unchanged after failed transfer"
        );
        assert_eq!(
            initial_balance2, final_balance2,
            "User2 balance should be unchanged after failed transfer"
        );
        
        println!("✅ Atomic transfer correctly rolled back on failure");
        println!("   Transfer amount: {} (more than available)", transfer_amount);
        println!("   User1 balance unchanged: {}", final_balance1);
        println!("   User2 balance unchanged: {}", final_balance2);
    }
    
    /// Test pour vérifier qu'il n'y a pas de panics avec des données invalides
    #[tokio::test]
    async fn test_no_panics_with_invalid_data() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/treichville_test".to_string());
        
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to connect to database");
        
        let wallet_service = WalletService::new(pool.clone());
        
        // Test with invalid UUID (non-existent user)
        let invalid_user_id = Uuid::new_v4();
        let result = wallet_service
            .debit(invalid_user_id, "XOF", Decimal::from(100), "test")
            .await;
        assert!(result.is_err(), "Should error for non-existent user");
        
        // Test with negative amount
        let user_id = Uuid::new_v4();
        wallet_service
            .get_or_create_wallet(user_id, "XOF")
            .await
            .expect("Failed to create wallet");
        
        let result = wallet_service
            .credit(user_id, "XOF", Decimal::from(-100), "negative-test")
            .await;
        assert!(result.is_err(), "Should error for negative amount");
        
        // Test with zero amount
        let result = wallet_service
            .transfer(user_id, user_id, "XOF", Decimal::ZERO, "zero-test")
            .await;
        assert!(result.is_err(), "Should error for zero amount");
        
        // Test with same user transfer
        let result = wallet_service
            .transfer(user_id, user_id, "XOF", Decimal::from(100), "same-user")
            .await;
        assert!(result.is_err(), "Should error for same user transfer");
        
        println!("✅ No panics detected with invalid data");
        println!("   All invalid operations returned proper errors");
    }
    
    /// Test de concurrence sur les réservations
    #[tokio::test]
    async fn test_concurrent_reservations() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/treichville_test".to_string());
        
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await
            .expect("Failed to connect to database");
        
        let wallet_service = Arc::new(WalletService::new(pool.clone()));
        
        // Setup
        let user_id = Uuid::new_v4();
        let initial_balance = Decimal::from(1000);
        
        wallet_service
            .get_or_create_wallet(user_id, "XOF")
            .await
            .expect("Failed to create wallet");
        
        wallet_service
            .credit(user_id, "XOF", initial_balance, "initial")
            .await
            .expect("Failed to credit");
        
        // Concurrent reservations
        let num_threads = 50;
        let reservation_amount = Decimal::from(30); // Total would be 1500, but we only have 1000
        
        let barrier = Arc::new(Barrier::new(num_threads));
        let mut handles = vec![];
        
        for i in 0..num_threads {
            let wallet_service = wallet_service.clone();
            let barrier = barrier.clone();
            
            let handle = tokio::spawn(async move {
                barrier.wait().await;
                
                wallet_service
                    .reserve(user_id, "XOF", reservation_amount, &format!("reserve-{}", i))
                    .await
                    .is_ok()
            });
            
            handles.push(handle);
        }
        
        let mut successful_reserves = 0;
        for handle in handles {
            if handle.await.expect("Thread panicked") {
                successful_reserves += 1;
            }
        }
        
        // Check that we didn't over-reserve
        let balance = wallet_service
            .get_balance(user_id, "XOF")
            .await
            .expect("Failed to get balance");
        
        let total_reserved = Decimal::from(successful_reserves) * reservation_amount;
        assert_eq!(
            balance.reserved_balance, total_reserved,
            "Reserved balance mismatch"
        );
        assert!(
            balance.available_balance >= Decimal::ZERO,
            "Available balance went negative!"
        );
        
        println!("✅ Concurrent reservations handled correctly");
        println!("   Successful reservations: {}/{}", successful_reserves, num_threads);
        println!("   Total reserved: {}", total_reserved);
        println!("   Available balance: {}", balance.available_balance);
    }
}