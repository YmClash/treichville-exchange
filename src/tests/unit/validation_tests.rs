use crate::shared::validation::{
    validate_email, validate_phone, validate_currency_code,
    validate_amount, validate_password_strength,
};
use rust_decimal::Decimal;

#[test]
fn test_email_validation() {
    // Valid emails
    assert!(validate_email("user@example.com").is_ok());
    assert!(validate_email("john.doe@company.co.uk").is_ok());
    assert!(validate_email("test+tag@gmail.com").is_ok());
    assert!(validate_email("user_123@test-domain.org").is_ok());
    
    // Invalid emails
    assert!(validate_email("invalid").is_err());
    assert!(validate_email("@example.com").is_err());
    assert!(validate_email("user@").is_err());
    assert!(validate_email("user @example.com").is_err());
    assert!(validate_email("user@example").is_err());
    assert!(validate_email("").is_err());
}

#[test]
fn test_phone_validation() {
    // Valid Ivory Coast phone numbers
    assert!(validate_phone("+2250708090807").is_ok());
    assert!(validate_phone("+2250101020304").is_ok());
    assert!(validate_phone("+2250505060708").is_ok());
    assert!(validate_phone("+2250777888999").is_ok());
    
    // Invalid phone numbers
    assert!(validate_phone("0708090807").is_err());     // Missing country code
    assert!(validate_phone("+225070809080").is_err());  // Too short
    assert!(validate_phone("+22507080908077").is_err()); // Too long
    assert!(validate_phone("+3361234567").is_err());     // Wrong country code
    assert!(validate_phone("not a phone").is_err());
    assert!(validate_phone("").is_err());
}

#[test]
fn test_currency_code_validation() {
    // Valid currency codes
    assert!(validate_currency_code("XOF").is_ok());
    assert!(validate_currency_code("EUR").is_ok());
    assert!(validate_currency_code("USD").is_ok());
    assert!(validate_currency_code("GBP").is_ok());
    assert!(validate_currency_code("BTC").is_ok());
    
    // Invalid currency codes
    assert!(validate_currency_code("EURO").is_err()); // Too long
    assert!(validate_currency_code("EU").is_err());   // Too short
    assert!(validate_currency_code("123").is_err());  // Numbers only
    assert!(validate_currency_code("").is_err());
    assert!(validate_currency_code("xof").is_err());  // Must be uppercase
}

#[test]
fn test_amount_validation() {
    // Valid amounts
    assert!(validate_amount(Decimal::from(100)).is_ok());
    assert!(validate_amount(Decimal::from(1)).is_ok());
    assert!(validate_amount(Decimal::from_str("0.01").unwrap()).is_ok());
    assert!(validate_amount(Decimal::from_str("999999.99").unwrap()).is_ok());
    
    // Invalid amounts
    assert!(validate_amount(Decimal::ZERO).is_err());
    assert!(validate_amount(Decimal::from(-100)).is_err());
    assert!(validate_amount(Decimal::from_str("-0.01").unwrap()).is_err());
}

#[test]
fn test_password_strength() {
    // Strong passwords
    assert!(validate_password_strength("SecurePass123!").is_ok());
    assert!(validate_password_strength("MyP@ssw0rd2024").is_ok());
    assert!(validate_password_strength("Complex!ty9").is_ok());
    assert!(validate_password_strength("Tr31chv!ll3_Exch@ng3").is_ok());
    
    // Weak passwords
    assert!(validate_password_strength("short").is_err());          // Too short
    assert!(validate_password_strength("password123").is_err());    // No uppercase
    assert!(validate_password_strength("PASSWORD123").is_err());    // No lowercase
    assert!(validate_password_strength("Password").is_err());       // No numbers
    assert!(validate_password_strength("Password123").is_err());    // No special chars
    assert!(validate_password_strength("").is_err());
}

#[test]
fn test_ivorian_phone_formats() {
    // Test different Ivorian operators
    let orange_numbers = vec![
        "+2250708090807",
        "+2250777888999",
        "+2250747584932",
    ];
    
    let mtn_numbers = vec![
        "+2250505060708",
        "+2250545678901",
        "+2250444555666",
    ];
    
    let moov_numbers = vec![
        "+2250101020304",
        "+2250141516171",
        "+2250202030405",
    ];
    
    for number in orange_numbers {
        assert!(validate_phone(number).is_ok(), "Orange number {} should be valid", number);
    }
    
    for number in mtn_numbers {
        assert!(validate_phone(number).is_ok(), "MTN number {} should be valid", number);
    }
    
    for number in moov_numbers {
        assert!(validate_phone(number).is_ok(), "Moov number {} should be valid", number);
    }
}

#[test]
fn test_transaction_limits() {
    use crate::shared::validation::validate_transaction_limits;
    
    // Within daily limits
    assert!(validate_transaction_limits(
        Decimal::from(100000),
        Decimal::from(500000),
    ).is_ok());
    
    // Exceeds daily limit (assuming 10M XOF)
    assert!(validate_transaction_limits(
        Decimal::from(15000000),
        Decimal::from(15000000),
    ).is_err());
    
    // Single transaction too large (assuming 5M XOF max)
    assert!(validate_transaction_limits(
        Decimal::from(6000000),
        Decimal::from(6000000),
    ).is_err());
}

#[test]
fn test_idempotency_key_validation() {
    use uuid::Uuid;
    
    // Valid UUIDs
    let valid_key = Uuid::new_v4().to_string();
    assert!(validate_idempotency_key(&valid_key).is_ok());
    
    // Invalid formats
    assert!(validate_idempotency_key("not-a-uuid").is_err());
    assert!(validate_idempotency_key("").is_err());
    assert!(validate_idempotency_key("12345").is_err());
}

#[test]
fn test_rate_spread_validation() {
    // Valid spreads (buy < sell)
    assert!(validate_rate_spread(
        Decimal::from(650),
        Decimal::from(660),
    ).is_ok());
    
    // Invalid spreads
    assert!(validate_rate_spread(
        Decimal::from(660),
        Decimal::from(650),
    ).is_err()); // Buy > Sell
    
    assert!(validate_rate_spread(
        Decimal::from(650),
        Decimal::from(650),
    ).is_err()); // Buy = Sell
    
    // Excessive spread (>10%)
    assert!(validate_rate_spread(
        Decimal::from(600),
        Decimal::from(700),
    ).is_err());
}

#[test]
fn test_confirmation_code_format() {
    // Valid 6-digit codes
    assert!(validate_confirmation_code("123456").is_ok());
    assert!(validate_confirmation_code("000001").is_ok());
    assert!(validate_confirmation_code("999999").is_ok());
    
    // Invalid codes
    assert!(validate_confirmation_code("12345").is_err());   // Too short
    assert!(validate_confirmation_code("1234567").is_err()); // Too long
    assert!(validate_confirmation_code("ABCDEF").is_err());  // Letters
    assert!(validate_confirmation_code("").is_err());
}

// Helper validation functions (to be implemented in validation module)
fn validate_idempotency_key(key: &str) -> Result<(), String> {
    uuid::Uuid::parse_str(key)
        .map(|_| ())
        .map_err(|_| "Invalid UUID format".to_string())
}

fn validate_rate_spread(buy_rate: Decimal, sell_rate: Decimal) -> Result<(), String> {
    if buy_rate >= sell_rate {
        return Err("Buy rate must be less than sell rate".to_string());
    }
    
    let spread = sell_rate - buy_rate;
    let spread_percentage = (spread / buy_rate) * Decimal::from(100);
    
    if spread_percentage > Decimal::from(10) {
        return Err("Spread exceeds 10%".to_string());
    }
    
    Ok(())
}

fn validate_confirmation_code(code: &str) -> Result<(), String> {
    if code.len() != 6 {
        return Err("Code must be 6 digits".to_string());
    }
    
    if !code.chars().all(|c| c.is_ascii_digit()) {
        return Err("Code must contain only digits".to_string());
    }
    
    Ok(())
}

use rust_decimal::prelude::FromStr;