use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use sqlx::{PgPool, postgres::PgQueryResult};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use crate::shared::errors::AppError;
use tracing::{info, warn, error};
use std::collections::HashMap;

/// Types d'événements de sécurité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    SqlInjectionAttempt,
    AuthenticationFailed,
    AuthorizationDenied,
    RateLimitExceeded,
    SuspiciousActivity,
    DataBreach,
    AccountLocked,
    PasswordChanged,
    TwoFactorEnabled,
    TwoFactorDisabled,
}

/// Niveau de sévérité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeverityLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Structure pour un événement de sécurité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub id: Uuid,
    pub event_type: SecurityEventType,
    pub severity: SeverityLevel,
    pub user_id: Option<Uuid>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub path: String,
    pub method: String,
    pub payload: Option<String>,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Service d'audit de sécurité
pub struct SecurityAuditService {
    db: Arc<PgPool>,
    cache: Arc<RwLock<HashMap<String, Vec<SecurityEvent>>>>, // Cache par IP
}

impl SecurityAuditService {
    pub fn new(db: Arc<PgPool>) -> Self {
        Self {
            db,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Enregistrer un événement de sécurité
    pub async fn log_event(&self, event: SecurityEvent) -> Result<(), AppError> {
        // Log dans la base de données
        let event_type_str = match event.event_type {
            SecurityEventType::SqlInjectionAttempt => "SqlInjectionAttempt",
            SecurityEventType::AuthenticationFailed => "AuthenticationFailed",
            SecurityEventType::AuthorizationDenied => "AuthorizationDenied",
            SecurityEventType::RateLimitExceeded => "RateLimitExceeded",
            SecurityEventType::SuspiciousActivity => "SuspiciousActivity",
            SecurityEventType::DataBreach => "DataBreach",
            SecurityEventType::AccountLocked => "AccountLocked",
            SecurityEventType::PasswordChanged => "PasswordChanged",
            SecurityEventType::TwoFactorEnabled => "TwoFactorEnabled",
            SecurityEventType::TwoFactorDisabled => "TwoFactorDisabled",
        }.to_string();
        
        let severity_str = match event.severity {
            SeverityLevel::Low => "Low",
            SeverityLevel::Medium => "Medium",
            SeverityLevel::High => "High",
            SeverityLevel::Critical => "Critical",
        }.to_string();
        
        let result = sqlx::query!(
            r#"
            INSERT INTO security_events (
                id, event_type, severity, user_id, ip_address,
                user_agent, path, method, payload, description,
                metadata, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
            event.id,
            event_type_str,
            severity_str,
            event.user_id,
            event.ip_address,
            event.user_agent,
            event.path,
            event.method,
            event.payload,
            event.description,
            event.metadata,
            event.created_at
        )
        .execute(&*self.db)
        .await;
        
        // Log même si l'insertion échoue
        match result {
            Ok(_) => {
                info!("Security event logged: {:?}", event.event_type);
            },
            Err(e) => {
                error!("Failed to log security event to database: {}", e);
            }
        }
        
        // Ajouter au cache pour analyse rapide
        let mut cache = self.cache.write().await;
        cache.entry(event.ip_address.clone())
            .or_insert_with(Vec::new)
            .push(event.clone());
        
        // Vérifier les patterns suspects
        self.check_suspicious_patterns(&event).await;
        
        Ok(())
    }
    
    /// Logger une tentative d'injection SQL
    pub async fn log_sql_injection(
        &self,
        ip_address: String,
        path: String,
        method: String,
        payload: String,
        user_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        let event = SecurityEvent {
            id: Uuid::new_v4(),
            event_type: SecurityEventType::SqlInjectionAttempt,
            severity: SeverityLevel::Critical,
            user_id,
            ip_address: ip_address.clone(),
            user_agent: None,
            path,
            method,
            payload: Some(payload.chars().take(1000).collect()), // Limiter la taille
            description: "SQL injection attempt detected and blocked".to_string(),
            metadata: None,
            created_at: Utc::now(),
        };
        
        error!(
            "🚨 SQL INJECTION BLOCKED - IP: {}, Path: {}", 
            ip_address, event.path
        );
        
        self.log_event(event).await?;
        
        // Vérifier si l'IP doit être bloquée
        self.check_ip_for_blocking(&ip_address).await?;
        
        Ok(())
    }
    
    /// Logger un échec d'authentification
    pub async fn log_auth_failure(
        &self,
        ip_address: String,
        username: String,
        reason: String,
    ) -> Result<(), AppError> {
        let event = SecurityEvent {
            id: Uuid::new_v4(),
            event_type: SecurityEventType::AuthenticationFailed,
            severity: SeverityLevel::Medium,
            user_id: None,
            ip_address: ip_address.clone(),
            user_agent: None,
            path: "/api/v1/auth/login".to_string(),
            method: "POST".to_string(),
            payload: Some(username),
            description: reason,
            metadata: None,
            created_at: Utc::now(),
        };
        
        warn!("Authentication failed from IP: {}", ip_address);
        
        self.log_event(event).await
    }
    
    /// Vérifier les patterns suspects
    async fn check_suspicious_patterns(&self, event: &SecurityEvent) {
        match event.event_type {
            SecurityEventType::SqlInjectionAttempt => {
                // Alerte immédiate pour les injections SQL
                self.send_security_alert(
                    "SQL Injection Attempt",
                    &format!("Critical security threat detected from IP: {}", event.ip_address),
                    &event.severity,
                ).await;
            },
            SecurityEventType::AuthenticationFailed => {
                // Vérifier les attaques par force brute
                let cache = self.cache.read().await;
                if let Some(events) = cache.get(&event.ip_address) {
                    let recent_failures = events.iter()
                        .filter(|e| {
                            matches!(e.event_type, SecurityEventType::AuthenticationFailed) &&
                            e.created_at > Utc::now() - chrono::Duration::minutes(5)
                        })
                        .count();
                    
                    if recent_failures >= 5 {
                        warn!("Possible brute force attack from IP: {}", event.ip_address);
                        self.send_security_alert(
                            "Brute Force Attack",
                            &format!("Multiple failed login attempts from IP: {}", event.ip_address),
                            &SeverityLevel::High,
                        ).await;
                    }
                }
            },
            _ => {}
        }
    }
    
    /// Vérifier si une IP doit être bloquée
    async fn check_ip_for_blocking(&self, ip_address: &str) -> Result<(), AppError> {
        // Compter les événements suspects récents
        let count = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM security_events
            WHERE ip_address = $1
              AND severity IN ('High', 'Critical')
              AND created_at > NOW() - INTERVAL '1 hour'
            "#,
            ip_address
        )
        .fetch_one(&*self.db)
        .await?;
        
        if count.count.unwrap_or(0) >= 10 {
            // Bloquer l'IP
            self.block_ip(ip_address).await?;
            
            error!("IP {} has been blocked due to suspicious activity", ip_address);
            
            self.send_security_alert(
                "IP Blocked",
                &format!("IP {} blocked after {} suspicious events", ip_address, count.count.unwrap_or(0)),
                &SeverityLevel::Critical,
            ).await;
        }
        
        Ok(())
    }
    
    /// Bloquer une adresse IP
    async fn block_ip(&self, ip_address: &str) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO blocked_ips (ip_address, reason, blocked_until)
            VALUES ($1, $2, NOW() + INTERVAL '24 hours')
            ON CONFLICT (ip_address) DO UPDATE
            SET blocked_at = NOW(),
                blocked_until = NOW() + INTERVAL '24 hours',
                block_count = blocked_ips.block_count + 1
            "#,
            ip_address,
            "Suspicious activity detected"
        )
        .execute(&*self.db)
        .await?;
        
        Ok(())
    }
    
    /// Vérifier si une IP est bloquée
    pub async fn is_ip_blocked(&self, ip_address: &str) -> Result<bool, AppError> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM blocked_ips
            WHERE ip_address = $1
              AND blocked_until > NOW()
              AND unblocked_at IS NULL
            "#,
            ip_address
        )
        .fetch_one(&*self.db)
        .await?;
        
        Ok(result.count.unwrap_or(0) > 0)
    }
    
    /// Envoyer une alerte de sécurité
    async fn send_security_alert(&self, title: &str, message: &str, severity: &SeverityLevel) {
        // TODO: Implémenter l'envoi réel (email, Slack, etc.)
        error!(
            "🚨 SECURITY ALERT 🚨\n\
            Title: {}\n\
            Severity: {:?}\n\
            Message: {}\n\
            Time: {}",
            title, severity, message, Utc::now()
        );
    }
    
    /// Obtenir les statistiques de sécurité
    pub async fn get_security_stats(&self, hours: i32) -> Result<SecurityStats, AppError> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) FILTER (WHERE event_type = 'SqlInjectionAttempt') as sql_injection_attempts,
                COUNT(*) FILTER (WHERE event_type = 'AuthenticationFailed') as auth_failures,
                COUNT(*) FILTER (WHERE severity = 'Critical') as critical_events,
                COUNT(*) FILTER (WHERE severity = 'High') as high_events,
                COUNT(DISTINCT ip_address) as unique_ips
            FROM security_events
            WHERE created_at > NOW() - INTERVAL '1 hour' * $1
            "#,
            hours as i64
        )
        .fetch_one(&*self.db)
        .await?;
        
        Ok(SecurityStats {
            sql_injection_attempts: stats.sql_injection_attempts.unwrap_or(0) as u32,
            auth_failures: stats.auth_failures.unwrap_or(0) as u32,
            critical_events: stats.critical_events.unwrap_or(0) as u32,
            high_events: stats.high_events.unwrap_or(0) as u32,
            unique_ips: stats.unique_ips.unwrap_or(0) as u32,
            period_hours: hours as u32,
        })
    }
    
    /// Nettoyer les vieux événements
    pub async fn cleanup_old_events(&self, days: i32) -> Result<u64, AppError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM security_events
            WHERE created_at < NOW() - INTERVAL '1 day' * $1
            "#,
            days as i64
        )
        .execute(&*self.db)
        .await?;
        
        Ok(result.rows_affected())
    }
}

/// Statistiques de sécurité
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityStats {
    pub sql_injection_attempts: u32,
    pub auth_failures: u32,
    pub critical_events: u32,
    pub high_events: u32,
    pub unique_ips: u32,
    pub period_hours: u32,
}

/// Migration SQL pour créer les tables d'audit
pub const SECURITY_AUDIT_MIGRATION: &str = r#"
-- Table pour les événements de sécurité
CREATE TABLE IF NOT EXISTS security_events (
    id UUID PRIMARY KEY,
    event_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    user_id UUID,
    ip_address VARCHAR(45) NOT NULL,
    user_agent TEXT,
    path VARCHAR(255),
    method VARCHAR(10),
    payload TEXT,
    description TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL,
    
    INDEX idx_security_events_ip (ip_address),
    INDEX idx_security_events_user (user_id),
    INDEX idx_security_events_created (created_at),
    INDEX idx_security_events_type (event_type),
    INDEX idx_security_events_severity (severity)
);

-- Table pour les IPs bloquées
CREATE TABLE IF NOT EXISTS blocked_ips (
    ip_address VARCHAR(45) PRIMARY KEY,
    reason TEXT,
    blocked_at TIMESTAMPTZ NOT NULL,
    blocked_until TIMESTAMPTZ NOT NULL,
    block_count INT DEFAULT 1,
    
    INDEX idx_blocked_ips_until (blocked_until)
);

-- Vue pour les statistiques rapides
CREATE OR REPLACE VIEW security_stats_hourly AS
SELECT 
    date_trunc('hour', created_at) as hour,
    event_type,
    severity,
    COUNT(*) as event_count,
    COUNT(DISTINCT ip_address) as unique_ips
FROM security_events
WHERE created_at > NOW() - INTERVAL '24 hours'
GROUP BY date_trunc('hour', created_at), event_type, severity
ORDER BY hour DESC;
"#;