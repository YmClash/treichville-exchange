use crate::{
    application::auth::{JwtService, AuthService, RegisterDto, LoginDto},
    tests::fixtures::{TestFixtures, helpers::*},
    domain::user::UserRole,
    shared::errors::AppError,
};
use std::sync::Arc;

#[tokio::test]
async fn test_jwt_service_token_generation() {
    let secret = TestFixtures::create_test_jwt_secret();
    let jwt_service = JwtService::new(secret, 900, 604800);
    
    let user_id = uuid::Uuid::new_v4();
    let role = UserRole::Client;
    
    // Generate access token
    let access_token = jwt_service
        .generate_access_token(user_id, &role)
        .expect("Failed to generate access token");
    
    assert!(!access_token.is_empty());
    
    // Verify the token
    let claims = jwt_service
        .verify_access_token(&access_token)
        .expect("Failed to verify access token");
    
    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.role, role.to_string());
}

#[tokio::test]
async fn test_jwt_service_refresh_token() {
    let secret = TestFixtures::create_test_jwt_secret();
    let jwt_service = JwtService::new(secret, 900, 604800);
    
    let user_id = uuid::Uuid::new_v4();
    
    // Generate refresh token
    let refresh_token = jwt_service
        .generate_refresh_token(user_id)
        .expect("Failed to generate refresh token");
    
    assert!(!refresh_token.is_empty());
    
    // Verify the refresh token
    let claims = jwt_service
        .verify_refresh_token(&refresh_token)
        .expect("Failed to verify refresh token");
    
    assert_eq!(claims.sub, user_id);
}

#[tokio::test]
async fn test_jwt_service_expired_token() {
    let secret = TestFixtures::create_test_jwt_secret();
    // Create service with 0 second expiry
    let jwt_service = JwtService::new(secret, 0, 0);
    
    let user_id = uuid::Uuid::new_v4();
    let role = UserRole::Client;
    
    let access_token = jwt_service
        .generate_access_token(user_id, &role)
        .expect("Failed to generate access token");
    
    // Wait for token to expire
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // Token should be expired
    let result = jwt_service.verify_access_token(&access_token);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_auth_service_register() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    let jwt_service = Arc::new(JwtService::new(
        TestFixtures::create_test_jwt_secret(),
        900,
        604800,
    ));
    
    let user_repo = Arc::new(crate::application::auth::UserRepository::new(db.clone()));
    let auth_service = AuthService::new(user_repo, jwt_service, redis);
    
    let register_dto = RegisterDto {
        email: "test@example.com".to_string(),
        phone: "+2250708090807".to_string(),
        password: "SecurePassword123!".to_string(),
        first_name: "Test".to_string(),
        last_name: "User".to_string(),
        role: UserRole::Client,
    };
    
    let result = auth_service.register(register_dto).await;
    assert!(result.is_ok());
    
    let response = result.unwrap();
    assert!(!response.access_token.is_empty());
    assert!(!response.refresh_token.is_empty());
    assert_eq!(response.user.email, "test@example.com");
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_auth_service_login_success() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    let jwt_service = Arc::new(JwtService::new(
        TestFixtures::create_test_jwt_secret(),
        900,
        604800,
    ));
    
    let user_repo = Arc::new(crate::application::auth::UserRepository::new(db.clone()));
    let auth_service = AuthService::new(user_repo.clone(), jwt_service, redis);
    
    // First register a user
    let register_dto = RegisterDto {
        email: "login@example.com".to_string(),
        phone: "+2250708090807".to_string(),
        password: "SecurePassword123!".to_string(),
        first_name: "Login".to_string(),
        last_name: "Test".to_string(),
        role: UserRole::Client,
    };
    
    auth_service.register(register_dto).await.unwrap();
    
    // Now try to login
    let login_dto = LoginDto {
        email: "login@example.com".to_string(),
        password: "SecurePassword123!".to_string(),
        two_factor_code: None,
    };
    
    let result = auth_service.login(login_dto).await;
    assert!(result.is_ok());
    
    let response = result.unwrap();
    assert!(!response.access_token.is_empty());
    assert!(!response.refresh_token.is_empty());
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_auth_service_login_wrong_password() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    let jwt_service = Arc::new(JwtService::new(
        TestFixtures::create_test_jwt_secret(),
        900,
        604800,
    ));
    
    let user_repo = Arc::new(crate::application::auth::UserRepository::new(db.clone()));
    let auth_service = AuthService::new(user_repo, jwt_service, redis);
    
    // First register a user
    let register_dto = RegisterDto {
        email: "wrong@example.com".to_string(),
        phone: "+2250708090807".to_string(),
        password: "SecurePassword123!".to_string(),
        first_name: "Wrong".to_string(),
        last_name: "Password".to_string(),
        role: UserRole::Client,
    };
    
    auth_service.register(register_dto).await.unwrap();
    
    // Try to login with wrong password
    let login_dto = LoginDto {
        email: "wrong@example.com".to_string(),
        password: "WrongPassword123!".to_string(),
        two_factor_code: None,
    };
    
    let result = auth_service.login(login_dto).await;
    assert!(result.is_err());
    
    if let Err(AppError::Unauthorized(msg)) = result {
        assert!(msg.contains("Invalid"));
    } else {
        panic!("Expected Unauthorized error");
    }
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_auth_service_duplicate_email() {
    let db = setup_test_db().await;
    let redis = Arc::new(setup_test_redis().await);
    let jwt_service = Arc::new(JwtService::new(
        TestFixtures::create_test_jwt_secret(),
        900,
        604800,
    ));
    
    let user_repo = Arc::new(crate::application::auth::UserRepository::new(db.clone()));
    let auth_service = AuthService::new(user_repo, jwt_service, redis);
    
    let register_dto = RegisterDto {
        email: "duplicate@example.com".to_string(),
        phone: "+2250708090807".to_string(),
        password: "SecurePassword123!".to_string(),
        first_name: "Duplicate".to_string(),
        last_name: "Test".to_string(),
        role: UserRole::Client,
    };
    
    // First registration should succeed
    let result1 = auth_service.register(register_dto.clone()).await;
    assert!(result1.is_ok());
    
    // Second registration with same email should fail
    let mut second_dto = register_dto;
    second_dto.phone = "+2250708090808".to_string(); // Different phone
    let result2 = auth_service.register(second_dto).await;
    assert!(result2.is_err());
    
    cleanup_test_db(&db).await;
}

#[tokio::test]
async fn test_password_hashing_verification() {
    use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
    use password_hash::SaltString;
    
    let password = "MySecurePassword123!";
    
    // Hash password
    let salt = SaltString::generate(&mut rand::thread_rng());
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();
    
    // Verify correct password
    let parsed_hash = PasswordHash::new(&password_hash).expect("Failed to parse hash");
    let verification = argon2.verify_password(password.as_bytes(), &parsed_hash);
    assert!(verification.is_ok());
    
    // Verify incorrect password
    let wrong_verification = argon2.verify_password(b"WrongPassword", &parsed_hash);
    assert!(wrong_verification.is_err());
}