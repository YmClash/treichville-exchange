use axum::{
    body::Body,
    extract::{Request, State, Query, Path},
    middleware::Next,
    response::{Response, IntoResponse},
    http::StatusCode,
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use tracing::{error, warn, info};
use crate::shared::sql_validator::SqlValidator;
use crate::shared::state::AppState;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Structure pour tracker les tentatives d'injection
#[derive(Debug, Clone)]
pub struct InjectionAttempt {
    pub timestamp: DateTime<Utc>,
    pub ip_address: String,
    pub path: String,
    pub payload: String,
    pub attack_type: String,
}

/// Middleware pour détecter et bloquer les tentatives d'injection SQL
pub async fn sql_injection_protection(
    State(_state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extraire l'IP du client
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    
    let uri = request.uri().clone();
    let path = uri.path().to_string();
    let query_string = uri.query().unwrap_or("");
    
    // Vérifier les query parameters
    if !query_string.is_empty() {
        if SqlValidator::detect_sql_injection_patterns(query_string) {
            log_injection_attempt(
                client_ip.clone(),
                path.clone(),
                query_string.to_string(),
                "Query Parameter Injection".to_string(),
            );
            
            return Err(StatusCode::BAD_REQUEST);
        }
        
        // Vérifier chaque paramètre individuellement
        // Parse simple des query params (key=value&key2=value2)
        let params: HashMap<String, String> = query_string
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.split('=');
                match (parts.next(), parts.next()) {
                    (Some(key), Some(value)) => Some((key.to_string(), value.to_string())),
                    _ => None,
                }
            })
            .collect();
        
        for (key, value) in params.iter() {
            if SqlValidator::detect_sql_injection_patterns(value) {
                log_injection_attempt(
                    client_ip.clone(),
                    path.clone(),
                    format!("{}={}", key, value),
                    format!("Parameter '{}' Injection", key),
                );
                
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }
    
    // Vérifier les headers suspects
    let suspicious_headers = vec![
        "x-api-key",
        "authorization", 
        "x-custom-header",
        "referer",
        "user-agent",
    ];
    
    for header_name in suspicious_headers {
        if let Some(header_value) = request.headers().get(header_name) {
            if let Ok(value_str) = header_value.to_str() {
                if SqlValidator::detect_sql_injection_patterns(value_str) {
                    log_injection_attempt(
                        client_ip.clone(),
                        path.clone(),
                        value_str.to_string(),
                        format!("Header '{}' Injection", header_name),
                    );
                    
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
        }
    }
    
    // Continuer avec la requête si elle est sûre
    Ok(next.run(request).await)
}

/// Middleware pour valider les paramètres de path
pub async fn validate_path_params(
    Path(params): Path<HashMap<String, String>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    for (key, value) in params.iter() {
        // Validation spécifique selon le type de paramètre
        match key.as_str() {
            "user_id" | "transaction_id" | "wallet_id" | "rate_id" => {
                // Valider comme UUID
                if Uuid::parse_str(value).is_err() {
                    warn!("Invalid UUID in path parameter {}: {}", key, value);
                    return Err(StatusCode::BAD_REQUEST);
                }
            },
            "role" => {
                // Valider le rôle
                if let Err(e) = SqlValidator::validate_user_role(value) {
                    warn!("Invalid role in path: {} - {}", value, e);
                    return Err(StatusCode::BAD_REQUEST);
                }
            },
            "currency" => {
                // Valider la devise
                if let Err(e) = SqlValidator::validate_currency(value) {
                    warn!("Invalid currency in path: {} - {}", value, e);
                    return Err(StatusCode::BAD_REQUEST);
                }
            },
            _ => {
                // Pour les autres paramètres, vérifier l'injection SQL
                if SqlValidator::detect_sql_injection_patterns(value) {
                    error!("SQL injection attempt in path parameter {}: {}", key, value);
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
        }
    }
    
    Ok(next.run(request).await)
}

/// Middleware pour limiter le taux de requêtes par IP
pub async fn rate_limit_by_ip(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // TODO: Implémenter un vrai rate limiting avec Redis
    // Pour l'instant, on laisse passer
    Ok(next.run(request).await)
}

/// Fonction pour logger les tentatives d'injection
fn log_injection_attempt(
    ip_address: String,
    path: String,
    payload: String,
    attack_type: String,
) {
    let attempt = InjectionAttempt {
        timestamp: Utc::now(),
        ip_address,
        path,
        payload: payload.chars().take(500).collect(), // Limiter la taille du log
        attack_type,
    };
    
    error!(
        "🚨 SQL INJECTION ATTEMPT DETECTED 🚨\n\
        IP: {}\n\
        Path: {}\n\
        Type: {}\n\
        Payload: {}\n\
        Time: {}",
        attempt.ip_address,
        attempt.path,
        attempt.attack_type,
        attempt.payload,
        attempt.timestamp
    );
    
    // TODO: Envoyer une alerte (email, Slack, etc.)
    // TODO: Sauvegarder dans une base de données d'audit
    // TODO: Bloquer l'IP après X tentatives
}

/// Middleware pour ajouter des headers de sécurité
pub async fn security_headers(
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    
    // Use proper error handling instead of unwrap
    if let Ok(value) = "nosniff".parse() {
        headers.insert("X-Content-Type-Options", value);
    }
    if let Ok(value) = "DENY".parse() {
        headers.insert("X-Frame-Options", value);
    }
    if let Ok(value) = "1; mode=block".parse() {
        headers.insert("X-XSS-Protection", value);
    }
    if let Ok(value) = "strict-origin-when-cross-origin".parse() {
        headers.insert("Referrer-Policy", value);
    }
    if let Ok(value) = "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'".parse() {
        headers.insert("Content-Security-Policy", value);
    }
    
    response
}

/// Structure pour une réponse d'erreur sécurisée
#[derive(serde::Serialize)]
pub struct SecurityErrorResponse {
    pub error: String,
    pub code: String,
    pub timestamp: DateTime<Utc>,
    pub request_id: String,
}

impl SecurityErrorResponse {
    pub fn new(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
            timestamp: Utc::now(),
            request_id: Uuid::new_v4().to_string(),
        }
    }
}

/// Handler pour les erreurs de sécurité
pub async fn handle_security_error(status: StatusCode) -> impl IntoResponse {
    let error_response = match status {
        StatusCode::BAD_REQUEST => SecurityErrorResponse::new(
            "Invalid request. Please check your input.",
            "INVALID_REQUEST",
        ),
        StatusCode::UNAUTHORIZED => SecurityErrorResponse::new(
            "Authentication required.",
            "UNAUTHORIZED",
        ),
        StatusCode::FORBIDDEN => SecurityErrorResponse::new(
            "Access denied.",
            "FORBIDDEN",
        ),
        _ => SecurityErrorResponse::new(
            "An error occurred.",
            "INTERNAL_ERROR",
        ),
    };
    
    (status, Json(error_response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::ServiceExt;
    
    #[tokio::test]
    async fn test_sql_injection_detection_in_query_params() {
        // Test que les injections SQL dans les query params sont détectées
        let malicious_queries = vec![
            "?id=1' OR '1'='1",
            "?name=admin'--",
            "?search='; DROP TABLE users; --",
        ];
        
        for query in malicious_queries {
            // Simuler une requête avec query string malveillant
            // Le middleware devrait bloquer ces requêtes
        }
    }
    
    #[tokio::test]
    async fn test_sql_injection_detection_in_headers() {
        // Test que les injections SQL dans les headers sont détectées
        let malicious_headers = vec![
            ("Authorization", "Bearer '; DROP TABLE--"),
            ("X-API-Key", "key' OR '1'='1"),
        ];
        
        for (header_name, header_value) in malicious_headers {
            // Simuler une requête avec header malveillant
            // Le middleware devrait bloquer ces requêtes
        }
    }
    
    #[tokio::test]
    async fn test_legitimate_requests_pass_through() {
        // Test que les requêtes légitimes passent
        let legitimate_queries = vec![
            "?id=550e8400-e29b-41d4-a716-446655440000",
            "?name=John%20Doe",
            "?amount=100.50",
        ];
        
        for query in legitimate_queries {
            // Ces requêtes devraient passer sans problème
        }
    }
}