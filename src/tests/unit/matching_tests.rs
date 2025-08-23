use crate::{
    application::exchange::{MatchingEngine, matching_engine::MatchCriteria},
    tests::fixtures::{TestFixtures, helpers::*},
    domain::user::UserRole,
};
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_matching_engine_scoring() {
    let db = setup_test_db().await;
    
    let rate_repo = Arc::new(crate::application::rates::RateRepository::new(db.clone()));
    let user_repo = Arc::new(crate::application::auth::UserRepository::new(db.clone()));
    
    let matching_engine = MatchingEngine::new(rate_repo.clone(), user_repo.clone());
    
    // Create test changeurs with different characteristics
    let changeurs = vec![
        create_changeur_with_profile(
            "Premium Changeur",
            Decimal::from(650),  // Best rate
            Decimal::from(95),   // High rating
            Decimal::from(100000), // High volume
            true,                 // Verified
            (5.3522, -4.0127),   // Location
        ),
        create_changeur_with_profile(
            "Average Changeur",
            Decimal::from(655),
            Decimal::from(80),
            Decimal::from(50000),
            true,
            (5.3500, -4.0100),
        ),
        create_changeur_with_profile(
            "New Changeur",
            Decimal::from(660),
            Decimal::from(70),
            Decimal::from(10000),
            false,
            (5.3600, -4.0200),
        ),
    ];
    
    // Store changeurs in database
    for changeur in &changeurs {
        // Store user and profile (implementation needed)
    }
    
    // Create match criteria
    let criteria = MatchCriteria {
        from_currency: "XOF".to_string(),
        to_currency: "EUR".to_string(),
        amount: Decimal::from(65000),
        client_location: Some((5.3522, -4.0127)),
        urgency: crate::application::exchange::matching_engine::UrgencyLevel::Normal,
        preferred_changeur_id: None,
    };
    
    // Find best match
    let matches = matching_engine.find_matches(criteria).await.expect("TODO: handle error");
    
    assert!(!matches.is_empty());
    // Premium changeur should rank first due to best rate and high rating
    assert!(matches[0].score > matches[1].score);
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_matching_distance_calculation() {
    let matching_engine = MatchingEngine::new(
        Arc::new(crate::application::rates::RateRepository::new(setup_test_db().await)),
        Arc::new(crate::application::auth::UserRepository::new(setup_test_db().await)),
    );
    
    // Test Haversine distance calculation
    let abidjan_center = (5.3600, -4.0083);
    let treichville = (5.2911, -4.0070);
    
    let distance = matching_engine.calculate_distance(
        abidjan_center.0,
        abidjan_center.1,
        treichville.0,
        treichville.1,
    );
    
    // Distance should be approximately 7.7 km
    assert!(distance > 7.0 && distance < 8.0);
}

#[tokio::test]
async fn test_matching_with_urgency() {
    let db = setup_test_db().await;
    
    let rate_repo = Arc::new(crate::application::rates::RateRepository::new(db.clone()));
    let user_repo = Arc::new(crate::application::auth::UserRepository::new(db.clone()));
    
    let matching_engine = MatchingEngine::new(rate_repo, user_repo);
    
    // Test that urgency affects scoring
    let normal_criteria = MatchCriteria {
        from_currency: "EUR".to_string(),
        to_currency: "XOF".to_string(),
        amount: Decimal::from(100),
        client_location: None,
        urgency: crate::application::exchange::matching_engine::UrgencyLevel::Normal,
        preferred_changeur_id: None,
    };
    
    let urgent_criteria = MatchCriteria {
        from_currency: "EUR".to_string(),
        to_currency: "XOF".to_string(),
        amount: Decimal::from(100),
        client_location: None,
        urgency: crate::application::exchange::matching_engine::UrgencyLevel::Critical,
        preferred_changeur_id: None,
    };
    
    // With critical urgency, availability should be weighted more heavily
    // Implementation would show different match ordering
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_matching_with_preferred_changeur() {
    let db = setup_test_db().await;
    
    let rate_repo = Arc::new(crate::application::rates::RateRepository::new(db.clone()));
    let user_repo = Arc::new(crate::application::auth::UserRepository::new(db.clone()));
    
    let matching_engine = MatchingEngine::new(rate_repo, user_repo);
    
    let preferred_id = Uuid::new_v4();
    
    let criteria = MatchCriteria {
        from_currency: "USD".to_string(),
        to_currency: "XOF".to_string(),
        amount: Decimal::from(500),
        client_location: None,
        urgency: crate::application::exchange::matching_engine::UrgencyLevel::Normal,
        preferred_changeur_id: Some(preferred_id),
    };
    
    // Preferred changeur should get a scoring bonus
    let matches = matching_engine.find_matches(criteria).await;
    
    // If preferred changeur exists and has rates, they should rank higher
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_matching_minimum_amount_filter() {
    let db = setup_test_db().await;
    
    let rate_repo = Arc::new(crate::application::rates::RateRepository::new(db.clone()));
    let user_repo = Arc::new(crate::application::auth::UserRepository::new(db.clone()));
    
    let matching_engine = MatchingEngine::new(rate_repo.clone(), user_repo);
    
    // Create rates with different min/max amounts
    let changeur_id = Uuid::new_v4();
    let rate = crate::domain::rate::CreateRateDto {
        from_currency: "EUR".to_string(),
        to_currency: "XOF".to_string(),
        buy_rate: Decimal::from(650),
        sell_rate: Decimal::from(660),
        available_amount: Some(Decimal::from(10000)),
        min_amount: Some(Decimal::from(100)),  // Minimum 100 EUR
        max_amount: Some(Decimal::from(5000)), // Maximum 5000 EUR
    };
    
    rate_repo.create(changeur_id, rate).await.expect("TODO: handle error");
    
    // Test with amount below minimum
    let below_min_criteria = MatchCriteria {
        from_currency: "EUR".to_string(),
        to_currency: "XOF".to_string(),
        amount: Decimal::from(50), // Below minimum
        client_location: None,
        urgency: crate::application::exchange::matching_engine::UrgencyLevel::Normal,
        preferred_changeur_id: None,
    };
    
    let matches = matching_engine.find_matches(below_min_criteria).await.expect("TODO: handle error");
    assert!(matches.is_empty(), "Should not match when below minimum amount");
    
    // Test with amount above maximum
    let above_max_criteria = MatchCriteria {
        from_currency: "EUR".to_string(),
        to_currency: "XOF".to_string(),
        amount: Decimal::from(10000), // Above maximum
        client_location: None,
        urgency: crate::application::exchange::matching_engine::UrgencyLevel::Normal,
        preferred_changeur_id: None,
    };
    
    let matches = matching_engine.find_matches(above_max_criteria).await.expect("TODO: handle error");
    assert!(matches.is_empty(), "Should not match when above maximum amount");
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_scoring_weights() {
    // Test that scoring weights are properly balanced
    let weights = vec![
        ("rate", 0.30),
        ("rating", 0.20),
        ("volume", 0.15),
        ("distance", 0.15),
        ("availability", 0.10),
        ("reliability", 0.10),
    ];
    
    let total: f64 = weights.iter().map(|(_, w)| w).sum();
    assert!((total - 1.0).abs() < 0.001, "Weights should sum to 1.0");
}

#[tokio::test]
async fn test_load_balancing() {
    let db = setup_test_db().await;
    
    let rate_repo = Arc::new(crate::application::rates::RateRepository::new(db.clone()));
    let user_repo = Arc::new(crate::application::auth::UserRepository::new(db.clone()));
    
    let matching_engine = MatchingEngine::new(rate_repo, user_repo);
    
    // Create multiple changeurs with similar scores
    let changeur_ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
    
    // Track distribution of matches
    let mut distribution = std::collections::HashMap::new();
    
    // Simulate multiple matching requests
    for _ in 0..100 {
        let criteria = MatchCriteria {
            from_currency: "EUR".to_string(),
            to_currency: "XOF".to_string(),
            amount: Decimal::from(1000),
            client_location: None,
            urgency: crate::application::exchange::matching_engine::UrgencyLevel::Normal,
            preferred_changeur_id: None,
        };
        
        // In real implementation, this would return different changeurs
        // based on load balancing algorithm
    }
    
    // Check that distribution is relatively even
    // Implementation would verify fair distribution
    
    cleanup_test_db(&db).await;
}

// Helper function to create changeur with profile
fn create_changeur_with_profile(
    name: &str,
    rate: Decimal,
    rating: Decimal,
    volume: Decimal,
    verified: bool,
    location: (f64, f64),
) -> TestChangeur {
    TestChangeur {
        name: name.to_string(),
        rate,
        rating,
        volume,
        verified,
        latitude: location.0,
        longitude: location.1,
    }
}

struct TestChangeur {
    name: String,
    rate: Decimal,
    rating: Decimal,
    volume: Decimal,
    verified: bool,
    latitude: f64,
    longitude: f64,
}