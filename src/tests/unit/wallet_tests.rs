use crate::{
    application::exchange::WalletService,
    domain::wallet::{WalletOperation, TransferRequest},
    tests::fixtures::{TestFixtures, helpers::*, assertions::*},
};
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_wallet_credit_operation() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    
    let wallet_service = WalletService::new(db.clone(), redis);
    
    let user_id = Uuid::new_v4();
    let currency = "XOF";
    let amount = Decimal::from(10000);
    
    // Credit wallet
    let operation = WalletOperation::Deposit {
        user_id,
        currency: currency.to_string(),
        amount,
        reference: "TEST_DEPOSIT".to_string(),
    };
    
    let result = wallet_service.process_operation(operation).await;
    assert!(result.is_ok());
    
    // Check balance
    let balance = wallet_service
        .get_balance_by_currency(user_id, currency)
        .await
        .expect("TODO: handle error");
    
    assert_eq!(balance.available_balance, amount);
    assert_eq!(balance.reserved_balance, Decimal::ZERO);
    assert_eq!(balance.total_balance, amount);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_wallet_debit_operation() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    
    let wallet_service = WalletService::new(db.clone(), redis);
    
    let user_id = Uuid::new_v4();
    let currency = "EUR";
    let initial_amount = Decimal::from(1000);
    let withdraw_amount = Decimal::from(300);
    
    // First deposit
    let deposit = WalletOperation::Deposit {
        user_id,
        currency: currency.to_string(),
        amount: initial_amount,
        reference: "INITIAL_DEPOSIT".to_string(),
    };
    wallet_service.process_operation(deposit).await.expect("TODO: handle error");
    
    // Then withdraw
    let withdraw = WalletOperation::Withdraw {
        user_id,
        currency: currency.to_string(),
        amount: withdraw_amount,
        reference: "TEST_WITHDRAW".to_string(),
    };
    let result = wallet_service.process_operation(withdraw).await;
    assert!(result.is_ok());
    
    // Check balance
    let balance = wallet_service
        .get_balance_by_currency(user_id, currency)
        .await
        .expect("TODO: handle error");
    
    assert_eq!(balance.available_balance, initial_amount - withdraw_amount);
    assert_eq!(balance.reserved_balance, Decimal::ZERO);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_wallet_insufficient_funds() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    
    let wallet_service = WalletService::new(db.clone(), redis);
    
    let user_id = Uuid::new_v4();
    let currency = "USD";
    let deposit_amount = Decimal::from(100);
    let withdraw_amount = Decimal::from(200); // More than available
    
    // Deposit
    let deposit = WalletOperation::Deposit {
        user_id,
        currency: currency.to_string(),
        amount: deposit_amount,
        reference: "DEPOSIT".to_string(),
    };
    wallet_service.process_operation(deposit).await.expect("TODO: handle error");
    
    // Try to withdraw more than available
    let withdraw = WalletOperation::Withdraw {
        user_id,
        currency: currency.to_string(),
        amount: withdraw_amount,
        reference: "OVER_WITHDRAW".to_string(),
    };
    
    let result = wallet_service.process_operation(withdraw).await;
    assert!(result.is_err());
    
    // Balance should remain unchanged
    let balance = wallet_service
        .get_balance_by_currency(user_id, currency)
        .await
        .expect("TODO: handle error");
    assert_eq!(balance.available_balance, deposit_amount);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_wallet_reserve_and_release() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    
    let wallet_service = WalletService::new(db.clone(), redis);
    
    let user_id = Uuid::new_v4();
    let currency = "XOF";
    let total_amount = Decimal::from(50000);
    let reserve_amount = Decimal::from(20000);
    let transaction_id = Uuid::new_v4();
    
    // Deposit initial funds
    let deposit = WalletOperation::Deposit {
        user_id,
        currency: currency.to_string(),
        amount: total_amount,
        reference: "INITIAL".to_string(),
    };
    wallet_service.process_operation(deposit).await.expect("TODO: handle error");
    
    // Reserve funds
    let reserve = WalletOperation::Reserve {
        user_id,
        currency: currency.to_string(),
        amount: reserve_amount,
        transaction_id,
    };
    wallet_service.process_operation(reserve).await.expect("TODO: handle error");
    
    // Check balances after reserve
    let balance = wallet_service
        .get_balance_by_currency(user_id, currency)
        .await
        .expect("TODO: handle error");
    
    assert_eq!(balance.available_balance, total_amount - reserve_amount);
    assert_eq!(balance.reserved_balance, reserve_amount);
    assert_eq!(balance.total_balance, total_amount);
    
    // Release funds
    let release = WalletOperation::Release {
        user_id,
        currency: currency.to_string(),
        amount: reserve_amount,
        transaction_id,
    };
    wallet_service.process_operation(release).await.expect("TODO: handle error");
    
    // Check balances after release
    let final_balance = wallet_service
        .get_balance_by_currency(user_id, currency)
        .await
        .expect("TODO: handle error");
    
    assert_eq!(final_balance.available_balance, total_amount);
    assert_eq!(final_balance.reserved_balance, Decimal::ZERO);
    assert_eq!(final_balance.total_balance, total_amount);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_wallet_transfer() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    
    let wallet_service = WalletService::new(db.clone(), redis);
    
    let sender_id = Uuid::new_v4();
    let receiver_id = Uuid::new_v4();
    let currency = "EUR";
    let initial_amount = Decimal::from(1000);
    let transfer_amount = Decimal::from(250);
    
    // Fund sender wallet
    let deposit = WalletOperation::Deposit {
        user_id: sender_id,
        currency: currency.to_string(),
        amount: initial_amount,
        reference: "SENDER_FUNDING".to_string(),
    };
    wallet_service.process_operation(deposit).await.expect("TODO: handle error");
    
    // Execute transfer
    let transfer = TransferRequest {
        from_user_id: sender_id,
        to_user_id: receiver_id,
        currency: currency.to_string(),
        amount: transfer_amount,
        reference: "TEST_TRANSFER".to_string(),
        metadata: None,
    };
    
    let result = wallet_service.transfer(transfer).await;
    assert!(result.is_ok());
    
    // Check sender balance
    let sender_balance = wallet_service
        .get_balance_by_currency(sender_id, currency)
        .await
        .expect("TODO: handle error");
    assert_eq!(sender_balance.available_balance, initial_amount - transfer_amount);
    
    // Check receiver balance
    let receiver_balance = wallet_service
        .get_balance_by_currency(receiver_id, currency)
        .await
        .expect("TODO: handle error");
    assert_eq!(receiver_balance.available_balance, transfer_amount);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_wallet_multi_currency() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    
    let wallet_service = WalletService::new(db.clone(), redis);
    
    let user_id = Uuid::new_v4();
    let currencies = vec![
        ("XOF", Decimal::from(100000)),
        ("EUR", Decimal::from(150)),
        ("USD", Decimal::from(200)),
    ];
    
    // Deposit in multiple currencies
    for (currency, amount) in &currencies {
        let deposit = WalletOperation::Deposit {
            user_id,
            currency: currency.to_string(),
            amount: *amount,
            reference: format!("DEPOSIT_{}", currency),
        };
        wallet_service.process_operation(deposit).await.expect("TODO: handle error");
    }
    
    // Get total balance
    let total_balance = wallet_service.get_balance(user_id).await.expect("TODO: handle error");
    
    assert_eq!(total_balance.wallets.len(), 3);
    
    // Verify each currency balance
    for (currency, expected_amount) in currencies {
        let balance = wallet_service
            .get_balance_by_currency(user_id, currency)
            .await
            .expect("TODO: handle error");
        assert_eq!(balance.available_balance, expected_amount);
    }
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_wallet_concurrent_operations() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    
    let wallet_service = Arc::new(WalletService::new(db.clone(), redis));
    
    let user_id = Uuid::new_v4();
    let currency = "XOF";
    let initial_amount = Decimal::from(10000);
    
    // Initial deposit
    let deposit = WalletOperation::Deposit {
        user_id,
        currency: currency.to_string(),
        amount: initial_amount,
        reference: "INITIAL".to_string(),
    };
    wallet_service.process_operation(deposit).await.expect("TODO: handle error");
    
    // Simulate concurrent withdrawals
    let concurrent_ops = 10;
    let withdraw_amount = Decimal::from(100);
    
    let mut handles = vec![];
    
    for i in 0..concurrent_ops {
        let service = wallet_service.clone();
        let handle = tokio::spawn(async move {
            let withdraw = WalletOperation::Withdraw {
                user_id,
                currency: currency.to_string(),
                amount: withdraw_amount,
                reference: format!("CONCURRENT_{}", i),
            };
            service.process_operation(withdraw).await
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All operations should succeed
    for result in results {
        assert!(result.is_ok());
        assert!(result.expect("TODO: handle error").is_ok());
    }
    
    // Final balance should be correct
    let final_balance = wallet_service
        .get_balance_by_currency(user_id, currency)
        .await
        .expect("TODO: handle error");
    
    let expected = initial_amount - (withdraw_amount * Decimal::from(concurrent_ops));
    assert_eq!(final_balance.available_balance, expected);
    
    cleanup_test_db(&db).await;
}