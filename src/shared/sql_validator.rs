use chrono::{DateTime, Utc};
use regex::Regex;
use crate::shared::errors::AppError;
use tracing::{warn, error};

/// Module de validation pour prévenir les injections SQL
pub struct SqlValidator;

impl SqlValidator {
    /// Valide qu'une chaîne de date est sûre et au bon format
    pub fn validate_datetime_string(date_str: &str) -> Result<DateTime<Utc>, AppError> {
        // Regex stricte pour ISO 8601
        let iso_regex = Regex::new(
            r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{1,9})?(Z|[+-]\d{2}:\d{2})$"
        ).map_err(|e| AppError::validation(format!("Regex compilation failed: {}", e)))?;
        
        if !iso_regex.is_match(date_str) {
            warn!("Invalid date format detected: {}", date_str);
            return Err(AppError::validation(
                format!("Invalid date format: {}", date_str)
            ));
        }
        
        // Détecter les patterns d'injection dans la date
        if Self::detect_sql_injection_patterns(date_str) {
            error!("SQL injection attempt detected in date: {}", date_str);
            return Err(AppError::validation(
                "Invalid date: potential security threat detected".to_string()
            ));
        }
        
        // Parser avec chrono pour validation supplémentaire
        let datetime = DateTime::parse_from_rfc3339(date_str)
            .map_err(|e| AppError::validation(format!("Invalid date: {}", e)))?
            .with_timezone(&Utc);
        
        Ok(datetime)
    }
    
    /// Valide qu'un rôle utilisateur est valide et sûr
    pub fn validate_user_role(role: &str) -> Result<&str, AppError> {
        // Vérifier d'abord les tentatives d'injection
        if Self::detect_sql_injection_patterns(role) {
            error!("SQL injection attempt detected in role: {}", role);
            return Err(AppError::validation(
                "Invalid role: potential security threat detected".to_string()
            ));
        }
        
        // Whitelist des rôles autorisés
        match role.to_lowercase().as_str() {
            "client" | "changeur" | "admin" => Ok(role),
            _ => {
                warn!("Invalid role attempted: {}", role);
                Err(AppError::validation(
                    format!("Invalid role: {}. Must be 'client', 'changeur', or 'admin'", role)
                ))
            }
        }
    }
    
    /// Valide les limites de pagination pour éviter les abus
    pub fn validate_pagination(limit: u32, offset: u32) -> Result<(i64, i64), AppError> {
        const MAX_LIMIT: u32 = 100;
        const MAX_OFFSET: u32 = 10000;
        
        if limit == 0 {
            return Err(AppError::validation(
                "Limit must be greater than 0".to_string()
            ));
        }
        
        if limit > MAX_LIMIT {
            warn!("Pagination limit {} exceeds maximum {}", limit, MAX_LIMIT);
            return Err(AppError::validation(
                format!("Limit {} exceeds maximum {}", limit, MAX_LIMIT)
            ));
        }
        
        if offset > MAX_OFFSET {
            warn!("Pagination offset {} exceeds maximum {}", offset, MAX_OFFSET);
            return Err(AppError::validation(
                format!("Offset {} exceeds maximum {}", offset, MAX_OFFSET)
            ));
        }
        
        Ok((limit as i64, offset as i64))
    }
    
    /// Valide une devise (currency code)
    pub fn validate_currency(currency: &str) -> Result<&str, AppError> {
        // Vérifier les tentatives d'injection
        if Self::detect_sql_injection_patterns(currency) {
            error!("SQL injection attempt detected in currency: {}", currency);
            return Err(AppError::validation(
                "Invalid currency: potential security threat detected".to_string()
            ));
        }
        
        // Whitelist des devises supportées
        match currency.to_uppercase().as_str() {
            "XOF" | "EUR" | "USD" | "GBP" | "BTC" => Ok(currency),
            _ => {
                warn!("Invalid currency attempted: {}", currency);
                Err(AppError::validation(
                    format!("Invalid currency: {}. Supported: XOF, EUR, USD, GBP, BTC", currency)
                ))
            }
        }
    }
    
    /// Détecte les patterns d'injection SQL courants
    pub fn detect_sql_injection_patterns(input: &str) -> bool {
        // Liste des patterns dangereux
        let dangerous_patterns = vec![
            // Commandes SQL dangereuses
            r"(\b(DROP|DELETE|INSERT|UPDATE|ALTER|CREATE|TRUNCATE|REPLACE|MERGE)\b)",
            // Commentaires SQL
            r"(--|#|\/\*|\*\/)",
            // Caractères d'échappement et spéciaux
            r"(;|\\x00|\\n|\\r|\\x1a|\\Z)",
            // UNION attacks
            r"(\bUNION\b.*\bSELECT\b)",
            // Boolean-based blind SQL injection
            r"(\bOR\b.*=.*)",
            r"(\bAND\b.*=.*)",
            // Apostrophes non échappées avec logique
            r"('.*\b(OR|AND)\b.*')",
            // Time-based blind SQL injection
            r"(\b(SLEEP|WAITFOR|DELAY|BENCHMARK)\b)",
            // Functions dangereuses
            r"(\b(EXEC|EXECUTE|CAST|CONVERT|CHAR|NCHAR|VARCHAR|NVARCHAR)\b)",
            // System procedures
            r"(xp_|sp_|0x)",
            // Stacked queries
            r"(;\s*(SELECT|INSERT|UPDATE|DELETE|DROP|CREATE|ALTER))",
        ];
        
        let input_upper = input.to_uppercase();
        
        for pattern in dangerous_patterns {
            let regex = match Regex::new(pattern) {
                Ok(r) => r,
                Err(e) => {
                    error!("Regex compilation failed: {}", e);
                    continue;
                }
            };
            
            if regex.is_match(&input_upper) {
                warn!("SQL injection pattern detected: {} matches pattern: {}", input, pattern);
                return true;
            }
        }
        
        // Vérifier aussi les caractères suspects multiples
        let suspicious_chars = vec!['\'', '"', ';', '-', '/', '*', '=', '\\'];
        let suspicious_count = suspicious_chars.iter()
            .filter(|&&c| input.contains(c))
            .count();
        
        if suspicious_count >= 3 {
            warn!("Multiple suspicious characters detected in: {}", input);
            return true;
        }
        
        false
    }
    
    /// Valide un identifiant (UUID ou ID numérique)
    pub fn validate_identifier(id: &str) -> Result<&str, AppError> {
        // Vérifier les tentatives d'injection
        if Self::detect_sql_injection_patterns(id) {
            error!("SQL injection attempt detected in identifier: {}", id);
            return Err(AppError::validation(
                "Invalid identifier: potential security threat detected".to_string()
            ));
        }
        
        // Regex pour UUID v4
        let uuid_regex = Regex::new(
            r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$"
        ).unwrap();
        
        // Regex pour ID numérique
        let numeric_regex = Regex::new(r"^\d+$").unwrap();
        
        if uuid_regex.is_match(id) || numeric_regex.is_match(id) {
            Ok(id)
        } else {
            warn!("Invalid identifier format: {}", id);
            Err(AppError::validation(
                "Invalid identifier format".to_string()
            ))
        }
    }
    
    /// Échappe les caractères spéciaux SQL (à utiliser en dernier recours uniquement)
    /// Préférer TOUJOURS les requêtes paramétrées
    pub fn escape_sql_string(input: &str) -> String {
        warn!("Using SQL string escaping - consider using parameterized queries instead");
        
        input
            .replace('\'', "''")
            .replace('\\', "\\\\")
            .replace('\0', "\\0")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\x1a', "\\Z")
    }
    
    /// Valide un montant décimal
    pub fn validate_amount(amount_str: &str) -> Result<rust_decimal::Decimal, AppError> {
        use rust_decimal::Decimal;
        use std::str::FromStr;
        
        // Vérifier les tentatives d'injection
        if Self::detect_sql_injection_patterns(amount_str) {
            error!("SQL injection attempt detected in amount: {}", amount_str);
            return Err(AppError::validation(
                "Invalid amount: potential security threat detected".to_string()
            ));
        }
        
        // Parser le montant
        let amount = Decimal::from_str(amount_str)
            .map_err(|_| AppError::validation("Invalid decimal amount".to_string()))?;
        
        // Vérifier que le montant est positif
        if amount.is_sign_negative() {
            return Err(AppError::validation(
                "Amount must be positive".to_string()
            ));
        }
        
        // Vérifier les limites raisonnables (100 milliards max)
        let max_amount = Decimal::from_str("100000000000").unwrap();
        if amount > max_amount {
            return Err(AppError::validation(
                "Amount exceeds maximum allowed value".to_string()
            ));
        }
        
        Ok(amount)
    }
    
    /// Nettoie et valide un numéro de téléphone
    pub fn validate_phone_number(phone: &str) -> Result<String, AppError> {
        // Vérifier les tentatives d'injection
        if Self::detect_sql_injection_patterns(phone) {
            error!("SQL injection attempt detected in phone: {}", phone);
            return Err(AppError::validation(
                "Invalid phone number: potential security threat detected".to_string()
            ));
        }
        
        // Garder seulement les chiffres et le + au début
        let cleaned = phone.chars()
            .enumerate()
            .filter(|(i, c)| c.is_ascii_digit() || (*i == 0 && *c == '+'))
            .map(|(_, c)| c)
            .collect::<String>();
        
        // Validation basique de la longueur
        if cleaned.len() < 8 || cleaned.len() > 15 {
            return Err(AppError::validation(
                "Phone number must be between 8 and 15 digits".to_string()
            ));
        }
        
        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sql_injection_detection() {
        // Cas qui doivent être détectés comme injections
        assert!(SqlValidator::detect_sql_injection_patterns("'; DROP TABLE users; --"));
        assert!(SqlValidator::detect_sql_injection_patterns("' OR '1'='1"));
        assert!(SqlValidator::detect_sql_injection_patterns("admin'--"));
        assert!(SqlValidator::detect_sql_injection_patterns("' UNION SELECT * FROM users"));
        assert!(SqlValidator::detect_sql_injection_patterns("1; DELETE FROM wallets"));
        assert!(SqlValidator::detect_sql_injection_patterns("' OR 1=1--"));
        assert!(SqlValidator::detect_sql_injection_patterns("'; EXEC xp_cmdshell('dir'); --"));
        assert!(SqlValidator::detect_sql_injection_patterns("SLEEP(5)"));
        assert!(SqlValidator::detect_sql_injection_patterns("WAITFOR DELAY '00:00:05'"));
        
        // Cas normaux qui ne doivent pas être détectés
        assert!(!SqlValidator::detect_sql_injection_patterns("normal_user_input"));
        assert!(!SqlValidator::detect_sql_injection_patterns("john.doe@example.com"));
        assert!(!SqlValidator::detect_sql_injection_patterns("2024-01-01"));
        assert!(!SqlValidator::detect_sql_injection_patterns("client"));
        assert!(!SqlValidator::detect_sql_injection_patterns("123456789"));
    }
    
    #[test]
    fn test_datetime_validation() {
        // Dates valides
        assert!(SqlValidator::validate_datetime_string("2024-01-01T12:00:00Z").is_ok());
        assert!(SqlValidator::validate_datetime_string("2024-01-01T12:00:00.123Z").is_ok());
        assert!(SqlValidator::validate_datetime_string("2024-01-01T12:00:00+02:00").is_ok());
        
        // Dates invalides
        assert!(SqlValidator::validate_datetime_string("invalid date").is_err());
        assert!(SqlValidator::validate_datetime_string("2024-01-01'; DROP TABLE--").is_err());
        assert!(SqlValidator::validate_datetime_string("2024-13-01T12:00:00Z").is_err());
    }
    
    #[test]
    fn test_role_validation() {
        // Rôles valides
        assert_eq!(SqlValidator::validate_user_role("client").unwrap(), "client");
        assert_eq!(SqlValidator::validate_user_role("changeur").unwrap(), "changeur");
        assert_eq!(SqlValidator::validate_user_role("admin").unwrap(), "admin");
        assert_eq!(SqlValidator::validate_user_role("CLIENT").unwrap(), "CLIENT");
        
        // Rôles invalides
        assert!(SqlValidator::validate_user_role("superadmin").is_err());
        assert!(SqlValidator::validate_user_role("'; DROP TABLE users; --").is_err());
        assert!(SqlValidator::validate_user_role("admin' OR '1'='1").is_err());
    }
    
    #[test]
    fn test_pagination_validation() {
        // Pagination valide
        assert_eq!(SqlValidator::validate_pagination(10, 0).unwrap(), (10, 0));
        assert_eq!(SqlValidator::validate_pagination(50, 100).unwrap(), (50, 100));
        assert_eq!(SqlValidator::validate_pagination(100, 0).unwrap(), (100, 0));
        
        // Pagination invalide
        assert!(SqlValidator::validate_pagination(0, 0).is_err());
        assert!(SqlValidator::validate_pagination(101, 0).is_err());
        assert!(SqlValidator::validate_pagination(10, 10001).is_err());
    }
    
    #[test]
    fn test_currency_validation() {
        // Devises valides
        assert_eq!(SqlValidator::validate_currency("XOF").unwrap(), "XOF");
        assert_eq!(SqlValidator::validate_currency("eur").unwrap(), "eur");
        assert_eq!(SqlValidator::validate_currency("USD").unwrap(), "USD");
        
        // Devises invalides
        assert!(SqlValidator::validate_currency("ABC").is_err());
        assert!(SqlValidator::validate_currency("XOF'; DROP TABLE--").is_err());
    }
    
    #[test]
    fn test_identifier_validation() {
        // UUID valide
        assert!(SqlValidator::validate_identifier("550e8400-e29b-41d4-a716-446655440000").is_ok());
        
        // ID numérique valide
        assert!(SqlValidator::validate_identifier("123456").is_ok());
        
        // Identifiants invalides
        assert!(SqlValidator::validate_identifier("not-a-uuid").is_err());
        assert!(SqlValidator::validate_identifier("123'; DROP TABLE--").is_err());
    }
    
    #[test]
    fn test_amount_validation() {
        use rust_decimal::Decimal;
        use std::str::FromStr;
        
        // Montants valides
        assert_eq!(
            SqlValidator::validate_amount("100.50").unwrap(),
            Decimal::from_str("100.50").unwrap()
        );
        assert_eq!(
            SqlValidator::validate_amount("1000000").unwrap(),
            Decimal::from_str("1000000").unwrap()
        );
        
        // Montants invalides
        assert!(SqlValidator::validate_amount("-100").is_err());
        assert!(SqlValidator::validate_amount("100'; DROP TABLE--").is_err());
        assert!(SqlValidator::validate_amount("999999999999999").is_err());
    }
    
    #[test]
    fn test_phone_validation() {
        // Numéros valides
        assert_eq!(SqlValidator::validate_phone_number("+22501234567").unwrap(), "+22501234567");
        assert_eq!(SqlValidator::validate_phone_number("0123456789").unwrap(), "0123456789");
        
        // Numéros invalides
        assert!(SqlValidator::validate_phone_number("123").is_err());
        assert!(SqlValidator::validate_phone_number("'; DROP TABLE--").is_err());
    }
}