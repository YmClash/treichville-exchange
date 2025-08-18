use chrono::{DateTime, Utc};
use rand::{distributions::Alphanumeric, Rng};
use regex::Regex;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use uuid::Uuid;
use validator::ValidationError;

static PHONE_REGEX: OnceLock<Regex> = OnceLock::new();
static CURRENCY_REGEX: OnceLock<Regex> = OnceLock::new();

pub fn init_validators() {
    PHONE_REGEX.get_or_init(|| {
        Regex::new(r"^\+225[0-9]{10}$").expect("Invalid phone regex")
    });
    
    CURRENCY_REGEX.get_or_init(|| {
        Regex::new(r"^[A-Z]{3}$").expect("Invalid currency regex")
    });
}

pub fn generate_transaction_reference() -> String {
    let date = Utc::now().format("%Y%m%d");
    let random: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(5)
        .map(char::from)
        .collect();
    format!("TR-{}-{}", date, random.to_uppercase())
}

pub fn generate_idempotency_key() -> String {
    Uuid::new_v4().to_string()
}

pub fn generate_otp() -> String {
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(0..1000000))
}

pub fn validate_phone_number(phone: &str) -> Result<(), ValidationError> {
    let regex = PHONE_REGEX.get().expect("Phone regex not initialized");
    if !regex.is_match(phone) {
        return Err(ValidationError::new("Invalid Ivory Coast phone number format. Expected: +225XXXXXXXXXX"));
    }
    Ok(())
}

pub fn validate_currency_code(code: &str) -> Result<(), ValidationError> {
    let regex = CURRENCY_REGEX.get().expect("Currency regex not initialized");
    if !regex.is_match(code) {
        return Err(ValidationError::new("Invalid currency code format. Expected: 3 uppercase letters"));
    }
    Ok(())
}

pub fn validate_amount(amount: Decimal, min: Decimal, max: Decimal) -> Result<(), ValidationError> {
    if amount < min {
        return Err(ValidationError::new("Amount below minimum allowed"));
    }
    if amount > max {
        return Err(ValidationError::new("Amount exceeds maximum allowed"));
    }
    if amount.scale() > 2 {
        return Err(ValidationError::new("Amount can have maximum 2 decimal places"));
    }
    Ok(())
}

pub fn calculate_fee(amount: Decimal, fee_percentage: Decimal) -> Decimal {
    (amount * fee_percentage / Decimal::from(100)).round_dp(2)
}

pub fn calculate_total_with_fee(amount: Decimal, fee_percentage: Decimal) -> Decimal {
    amount + calculate_fee(amount, fee_percentage)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Money {
    pub amount: Decimal,
    pub currency: String,
}

impl Money {
    pub fn new(amount: Decimal, currency: impl Into<String>) -> Self {
        Self {
            amount: amount.round_dp(2),
            currency: currency.into().to_uppercase(),
        }
    }

    pub fn xof(amount: Decimal) -> Self {
        Self::new(amount, "XOF")
    }

    pub fn eur(amount: Decimal) -> Self {
        Self::new(amount, "EUR")
    }

    pub fn usd(amount: Decimal) -> Self {
        Self::new(amount, "USD")
    }

    pub fn validate(&self, min: Decimal, max: Decimal) -> Result<(), ValidationError> {
        validate_currency_code(&self.currency)?;
        validate_amount(self.amount, min, max)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl DateRange {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Self, ValidationError> {
        if start > end {
            return Err(ValidationError::new("Start date must be before end date"));
        }
        Ok(Self { start, end })
    }

    pub fn today() -> Self {
        let now = Utc::now();
        let start = now.date_naive().and_hms_opt(0, 0, 0)
            .expect("Valid time")
            .and_utc();
        let end = now.date_naive().and_hms_opt(23, 59, 59)
            .expect("Valid time")
            .and_utc();
        Self { start, end }
    }

    pub fn this_month() -> Self {
        let now = Utc::now();
        let start = now
            .date_naive()
            .with_day(1)
            .expect("Valid day")
            .and_hms_opt(0, 0, 0)
            .expect("Valid time")
            .and_utc();
        
        let next_month = if now.month() == 12 {
            now.with_year(now.year() + 1)
                .expect("Valid year")
                .with_month(1)
                .expect("Valid month")
        } else {
            now.with_month(now.month() + 1).expect("Valid month")
        };
        
        let end = next_month
            .date_naive()
            .with_day(1)
            .expect("Valid day")
            .and_hms_opt(0, 0, 0)
            .expect("Valid time")
            .and_utc()
            - chrono::Duration::seconds(1);
        
        Self { start, end }
    }

    pub fn contains(&self, datetime: DateTime<Utc>) -> bool {
        datetime >= self.start && datetime <= self.end
    }
}

pub fn mask_phone_number(phone: &str) -> String {
    if phone.len() < 10 {
        return "***".to_string();
    }
    let prefix = &phone[..6];
    let suffix = &phone[phone.len()-2..];
    format!("{}****{}", prefix, suffix)
}

pub fn mask_email(email: &str) -> String {
    if let Some(at_pos) = email.find('@') {
        let local = &email[..at_pos];
        let domain = &email[at_pos..];
        
        if local.len() <= 2 {
            format!("**{}", domain)
        } else {
            let visible = &local[..2];
            format!("{}***{}", visible, domain)
        }
    } else {
        "***".to_string()
    }
}

pub fn sanitize_input(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(1000)
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: u32,
    pub per_page: u32,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 20,
        }
    }
}

impl PaginationParams {
    pub fn offset(&self) -> u32 {
        (self.page.saturating_sub(1)) * self.per_page
    }

    pub fn limit(&self) -> u32 {
        self.per_page.min(100)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: u64, params: PaginationParams) -> Self {
        let total_pages = ((total as f64) / (params.per_page as f64)).ceil() as u32;
        Self {
            data,
            total,
            page: params.page,
            per_page: params.per_page,
            total_pages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_transaction_reference() {
        let reference = generate_transaction_reference();
        assert!(reference.starts_with("TR-"));
        assert_eq!(reference.len(), 14);
    }

    #[test]
    fn test_validate_phone_number() {
        init_validators();
        
        assert!(validate_phone_number("+2250123456789").is_ok());
        assert!(validate_phone_number("+2251234567890").is_ok());
        assert!(validate_phone_number("0123456789").is_err());
        assert!(validate_phone_number("+33123456789").is_err());
    }

    #[test]
    fn test_calculate_fee() {
        let amount = Decimal::from(1000);
        let fee_percentage = Decimal::from(2);
        let fee = calculate_fee(amount, fee_percentage);
        assert_eq!(fee, Decimal::from(20));
    }

    #[test]
    fn test_money_validation() {
        init_validators();
        
        let money = Money::xof(Decimal::from(5000));
        assert!(money.validate(Decimal::from(1000), Decimal::from(10000)).is_ok());
        
        let money = Money::xof(Decimal::from(500));
        assert!(money.validate(Decimal::from(1000), Decimal::from(10000)).is_err());
    }

    #[test]
    fn test_mask_phone_number() {
        assert_eq!(mask_phone_number("+2250123456789"), "+22501****89");
        assert_eq!(mask_phone_number("0123456789"), "012345****89");
    }
}