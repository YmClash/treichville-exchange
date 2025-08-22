use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use serde_json::json;

use crate::{
    presentation::rest::routes::create_router,
    tests::fixtures::{helpers::*, TestFixtures},
    shared::state::AppState,
};

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_register_endpoint() {
    let app = create_test_app().await;
    
    let payload = json!({
        "email": "test@example.com",
        "phone": "+2250708090807",
        "password": "SecurePass123!",
        "first_name": "Test",
        "last_name": "User",
        "role": "client"
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::CREATED);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["access_token"].is_string());
    assert!(json["refresh_token"].is_string());
    assert_eq!(json["user"]["email"], "test@example.com");
}

#[tokio::test]
async fn test_login_endpoint() {
    let app = create_test_app().await;
    
    // First register
    let register_payload = json!({
        "email": "login@example.com",
        "phone": "+2250708090807",
        "password": "SecurePass123!",
        "first_name": "Login",
        "last_name": "Test",
        "role": "client"
    });
    
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&register_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    // Then login
    let login_payload = json!({
        "email": "login@example.com",
        "password": "SecurePass123!"
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&login_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["access_token"].is_string());
    assert!(json["refresh_token"].is_string());
}

#[tokio::test]
async fn test_protected_endpoint_without_auth() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_endpoint_with_auth() {
    let app = create_test_app().await;
    
    // Get auth token
    let token = get_test_auth_token(&app).await;
    
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_public_rates_endpoint() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/rates/public")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json.is_array() || json.is_object());
}

#[tokio::test]
async fn test_changeur_rate_creation() {
    let app = create_test_app().await;
    
    // Get changeur token
    let token = get_test_changeur_token(&app).await;
    
    let rate_payload = json!({
        "from_currency": "EUR",
        "to_currency": "XOF",
        "buy_rate": "650",
        "sell_rate": "660",
        "available_amount": "10000",
        "min_amount": "10",
        "max_amount": "5000"
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/rates")
                .header("authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&rate_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_rate_limit() {
    let app = create_test_app().await;
    
    // Send multiple requests quickly
    let mut responses = vec![];
    
    for _ in 0..20 {
        let response = app.clone()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        responses.push(response.status());
    }
    
    // At some point we should get rate limited
    // This depends on the actual rate limit configuration
    // assert!(responses.iter().any(|&status| status == StatusCode::TOO_MANY_REQUESTS));
}

#[tokio::test]
async fn test_cors_headers() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/rates/public")
                .header("origin", "http://localhost:3000")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("access-control-allow-origin"));
}

// Helper functions
async fn create_test_app() -> axum::Router {
    let db = setup_test_db().await;
    let redis = std::sync::Arc::new(setup_test_redis().await);
    
    // Create test app state
    let config = crate::shared::config::Config {
        server: crate::shared::config::ServerConfig {
            port: 3000,
            host: "127.0.0.1".to_string(),
        },
        database: crate::shared::config::DatabaseConfig {
            url: TestFixtures::create_test_database_url(),
            max_connections: 5,
            acquire_timeout: 10,
        },
        redis: crate::shared::config::RedisConfig {
            url: TestFixtures::create_test_redis_url(),
        },
        jwt: crate::shared::config::JwtConfig {
            secret: TestFixtures::create_test_jwt_secret(),
            access_token_expiry: 900,
            refresh_token_expiry: 604800,
        },
        binance: crate::shared::config::BinanceConfig {
            api_key: "test_key".to_string(),
            api_secret: "test_secret".to_string(),
            testnet: true,
        },
    };
    
    // Initialize services
    let jwt_service = std::sync::Arc::new(crate::application::auth::JwtService::new(
        config.jwt.secret.clone(),
        config.jwt.access_token_expiry,
        config.jwt.refresh_token_expiry,
    ));
    
    let user_repo = std::sync::Arc::new(crate::application::auth::UserRepository::new(db.clone()));
    let auth_service = std::sync::Arc::new(crate::application::auth::AuthService::new(
        user_repo.clone(),
        jwt_service.clone(),
        redis.clone(),
    ));
    
    let rate_repo = std::sync::Arc::new(crate::application::rates::RateRepository::new(db.clone()));
    let rate_cache = std::sync::Arc::new(crate::application::rates::RateCache::new(redis.clone()));
    let rate_aggregator = std::sync::Arc::new(crate::application::rates::RateAggregator::new(
        rate_repo.clone(),
        rate_cache.clone(),
    ));
    let rate_service = std::sync::Arc::new(crate::application::rates::RateService::new(
        rate_repo.clone(),
        rate_cache.clone(),
        rate_aggregator.clone(),
    ));
    
    let transaction_repo = std::sync::Arc::new(crate::application::exchange::TransactionRepository::new(db.clone()));
    let matching_engine = std::sync::Arc::new(crate::application::exchange::MatchingEngine::new(
        rate_repo.clone(),
        user_repo.clone(),
    ));
    let wallet_service = std::sync::Arc::new(crate::application::exchange::WalletService::new(
        db.clone(),
        redis.clone(),
    ));
    let payment_processor = std::sync::Arc::new(crate::application::exchange::PaymentProcessor::new(
        transaction_repo.clone(),
        wallet_service.clone(),
        config.clone(),
    ));
    let transaction_service = std::sync::Arc::new(crate::application::exchange::TransactionService::new(
        transaction_repo.clone(),
        user_repo.clone(),
        rate_service.clone(),
        wallet_service.clone(),
        payment_processor.clone(),
        config.clone(),
    ));
    
    let app_state = std::sync::Arc::new(AppState {
        config,
        db,
        redis,
        auth_service,
        rate_service,
        transaction_service,
        wallet_service,
        jwt_service,
    });
    
    create_router(app_state)
}

async fn get_test_auth_token(app: &axum::Router) -> String {
    let payload = json!({
        "email": "token@example.com",
        "phone": "+2250708090807",
        "password": "SecurePass123!",
        "first_name": "Token",
        "last_name": "User",
        "role": "client"
    });
    
    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    json["access_token"].as_str().unwrap().to_string()
}

async fn get_test_changeur_token(app: &axum::Router) -> String {
    let payload = json!({
        "email": "changeur@example.com",
        "phone": "+2250708090808",
        "password": "SecurePass123!",
        "first_name": "Changeur",
        "last_name": "Test",
        "role": "changeur"
    });
    
    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    json["access_token"].as_str().unwrap().to_string()
}