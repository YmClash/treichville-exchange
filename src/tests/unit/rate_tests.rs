use crate::{
    application::rates::{RateService, RateRepository, RateCache, RateAggregator},
    domain::rate::{CreateRateDto, UpdateRateDto, ExchangeOperation},
    tests::fixtures::{TestFixtures, helpers::*, assertions::*},
};
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_rate_aggregator_calculations() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    
    let rate_repo = Arc::new(RateRepository::new(db.clone()));
    let rate_cache = Arc::new(RateCache::new(redis.clone()));
    let aggregator = RateAggregator::new(rate_repo.clone(), rate_cache.clone());
    
    // Create test changeurs
    let changeur1_id = Uuid::new_v4();
    let changeur2_id = Uuid::new_v4();
    let changeur3_id = Uuid::new_v4();
    
    // Create rates with different values
    let rates = vec![
        CreateRateDto {
            from_currency: "EUR".to_string(),
            to_currency: "XOF".to_string(),
            buy_rate: Decimal::from(650),
            sell_rate: Decimal::from(660),
            available_amount: Some(Decimal::from(10000)),
            min_amount: Some(Decimal::from(10)),
            max_amount: Some(Decimal::from(5000)),
        },
        CreateRateDto {
            from_currency: "EUR".to_string(),
            to_currency: "XOF".to_string(),
            buy_rate: Decimal::from(655),
            sell_rate: Decimal::from(665),
            available_amount: Some(Decimal::from(15000)),
            min_amount: Some(Decimal::from(10)),
            max_amount: Some(Decimal::from(5000)),
        },
        CreateRateDto {
            from_currency: "EUR".to_string(),
            to_currency: "XOF".to_string(),
            buy_rate: Decimal::from(645),
            sell_rate: Decimal::from(655),
            available_amount: Some(Decimal::from(20000)),
            min_amount: Some(Decimal::from(10)),
            max_amount: Some(Decimal::from(5000)),
        },
    ];
    
    // Insert rates
    for (i, (changeur_id, dto)) in vec![changeur1_id, changeur2_id, changeur3_id]
        .into_iter()
        .zip(rates.into_iter())
        .enumerate()
    {
        rate_repo.create(changeur_id, dto).await.expect("TODO: handle error");
    }
    
    // Test aggregation
    let aggregated = aggregator.aggregate_rates("EUR", "XOF").await.expect("TODO: handle error");
    
    // Check calculations
    assert_decimal_close(aggregated.average_buy, Decimal::from(650), Decimal::from(1));
    assert_decimal_close(aggregated.average_sell, Decimal::from(660), Decimal::from(1));
    assert_eq!(aggregated.best_buy, Decimal::from(645));
    assert_eq!(aggregated.best_sell, Decimal::from(655));
    assert_eq!(aggregated.median_buy, Decimal::from(650));
    assert_eq!(aggregated.median_sell, Decimal::from(660));
    assert_eq!(aggregated.total_volume, Decimal::from(45000));
    assert_eq!(aggregated.changeur_count, 3);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_rate_manipulation_detection() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    
    let rate_repo = Arc::new(RateRepository::new(db.clone()));
    
    // Create normal rates
    for i in 0..5 {
        let changeur_id = Uuid::new_v4();
        let rate = CreateRateDto {
            from_currency: "USD".to_string(),
            to_currency: "XOF".to_string(),
            buy_rate: Decimal::from(590 + i),
            sell_rate: Decimal::from(600 + i),
            available_amount: Some(Decimal::from(10000)),
            min_amount: Some(Decimal::from(10)),
            max_amount: Some(Decimal::from(5000)),
        };
        rate_repo.create(changeur_id, rate).await.expect("TODO: handle error");
    }
    
    // Test manipulation detection with outlier rate
    let manipulator_id = Uuid::new_v4();
    let outlier_rate = Decimal::from(700); // Way above average
    
    let is_manipulation = rate_repo
        .detect_manipulation(manipulator_id, "USD", "XOF", outlier_rate)
        .await
        .expect("TODO: handle error");
    
    assert!(is_manipulation, "Should detect rate manipulation");
    
    // Test normal rate
    let normal_rate = Decimal::from(595);
    let is_normal = rate_repo
        .detect_manipulation(manipulator_id, "USD", "XOF", normal_rate)
        .await
        .expect("TODO: handle error");
    
    assert!(!is_normal, "Should not flag normal rate as manipulation");
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_rate_cache_operations() {
    let redis = Arc::new(setup_test_redis().await);
    let mut redis_conn = redis.clone();
    let cache = RateCache::new(redis);
    
    let test_rate = TestFixtures::create_test_rate(Uuid::new_v4());
    
    // Test set and get
    cache.set_rate(&test_rate).await.expect("TODO: handle error");
    
    let cached = cache
        .get_rate(test_rate.id)
        .await
        .expect("TODO: handle error")
        .expect("Rate should be in cache");
    
    assert_rate_equal(&test_rate, &cached);
    
    // Test cache expiry (TTL)
    cache.set_with_ttl(&test_rate, 1).await.expect("TODO: handle error");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    let expired = cache.get_rate(test_rate.id).await.expect("TODO: handle error");
    assert!(expired.is_none(), "Rate should have expired");
    
    cleanup_test_redis(&mut redis_conn).await;
}

#[tokio::test]
async fn test_best_rate_calculation() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    
    let rate_repo = Arc::new(RateRepository::new(db.clone()));
    let rate_cache = Arc::new(RateCache::new(redis.clone()));
    let aggregator = Arc::new(RateAggregator::new(rate_repo.clone(), rate_cache.clone()));
    let rate_service = RateService::new(rate_repo, rate_cache, aggregator);
    
    // Create multiple rates
    let changeurs = vec![
        (Uuid::new_v4(), Decimal::from(650), Decimal::from(660)),
        (Uuid::new_v4(), Decimal::from(645), Decimal::from(655)), // Best rates
        (Uuid::new_v4(), Decimal::from(655), Decimal::from(665)),
    ];
    
    for (changeur_id, buy, sell) in changeurs {
        let dto = CreateRateDto {
            from_currency: "EUR".to_string(),
            to_currency: "XOF".to_string(),
            buy_rate: buy,
            sell_rate: sell,
            available_amount: Some(Decimal::from(50000)),
            min_amount: Some(Decimal::from(10)),
            max_amount: Some(Decimal::from(10000)),
        };
        rate_service.create_rate(changeur_id, dto).await.expect("TODO: handle error");
    }
    
    // Get best rates
    let best_rates = rate_service
        .get_best_rates("EUR", "XOF", Decimal::from(1000))
        .await
        .expect("TODO: handle error");
    
    assert!(!best_rates.rates.is_empty());
    // First rate should be the best one
    assert_eq!(best_rates.rates[0].sell_rate, Decimal::from(655));
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_quote_generation() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    
    let rate_repo = Arc::new(RateRepository::new(db.clone()));
    let rate_cache = Arc::new(RateCache::new(redis.clone()));
    let aggregator = Arc::new(RateAggregator::new(rate_repo.clone(), rate_cache.clone()));
    let rate_service = RateService::new(rate_repo, rate_cache, aggregator);
    
    // Create a rate
    let changeur_id = Uuid::new_v4();
    let dto = CreateRateDto {
        from_currency: "EUR".to_string(),
        to_currency: "XOF".to_string(),
        buy_rate: Decimal::from(650),
        sell_rate: Decimal::from(660),
        available_amount: Some(Decimal::from(50000)),
        min_amount: Some(Decimal::from(10)),
        max_amount: Some(Decimal::from(10000)),
    };
    rate_service.create_rate(changeur_id, dto).await.expect("TODO: handle error");
    
    // Generate quote for buying EUR with XOF
    let quote_request = crate::domain::rate::GetQuoteRequest {
        from_currency: "XOF".to_string(),
        to_currency: "EUR".to_string(),
        amount: Decimal::from(6600), // Should get 10 EUR
        operation: ExchangeOperation::Buy,
    };
    
    let quote = rate_service.get_quote(quote_request).await.expect("TODO: handle error");
    
    assert_eq!(quote.from_currency, "XOF");
    assert_eq!(quote.to_currency, "EUR");
    assert_eq!(quote.rate, Decimal::from(660)); // Sell rate from changeur perspective
    assert_decimal_close(quote.converted_amount, Decimal::from(10), Decimal::from(1));
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_spread_calculation() {
    let rate = TestFixtures::create_test_rate(Uuid::new_v4());
    
    let expected_spread = rate.sell_rate - rate.buy_rate;
    assert_eq!(rate.spread, expected_spread);
    
    let spread_percentage = (rate.spread / rate.mid_rate) * Decimal::from(100);
    assert!(spread_percentage > Decimal::ZERO);
    assert!(spread_percentage < Decimal::from(5)); // Reasonable spread
}

#[tokio::test]
async fn test_market_depth() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    
    let rate_repo = Arc::new(RateRepository::new(db.clone()));
    let rate_cache = Arc::new(RateCache::new(redis.clone()));
    let aggregator = Arc::new(RateAggregator::new(rate_repo.clone(), rate_cache.clone()));
    let rate_service = RateService::new(rate_repo, rate_cache, aggregator);
    
    // Create rates at different price levels
    let price_levels = vec![
        (Decimal::from(650), Decimal::from(10000)),
        (Decimal::from(655), Decimal::from(15000)),
        (Decimal::from(660), Decimal::from(20000)),
    ];
    
    for (i, (rate, volume)) in price_levels.iter().enumerate() {
        let dto = CreateRateDto {
            from_currency: "EUR".to_string(),
            to_currency: "XOF".to_string(),
            buy_rate: *rate - Decimal::from(5),
            sell_rate: *rate,
            available_amount: Some(*volume),
            min_amount: Some(Decimal::from(10)),
            max_amount: Some(*volume),
        };
        rate_service.create_rate(Uuid::new_v4(), dto).await.expect("TODO: handle error");
    }
    
    let depth = rate_service.get_market_depth("EUR", "XOF").await.expect("TODO: handle error");
    
    assert_eq!(depth.buy_orders.len(), 3);
    assert_eq!(depth.sell_orders.len(), 3);
    assert_eq!(depth.total_buy_volume, Decimal::from(45000));
    assert_eq!(depth.total_sell_volume, Decimal::from(45000));
    
    cleanup_test_db(&db).await;
}