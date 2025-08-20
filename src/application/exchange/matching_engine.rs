use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn, debug};
use uuid::Uuid;

use crate::{
    domain::{
        rate::ExchangeRate,
        transaction::{Transaction, TransactionType},
        ChangeurProfile,
    },
    shared::errors::AppError,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingCriteria {
    pub from_currency: String,
    pub to_currency: String,
    pub amount: Decimal,
    pub client_id: Uuid,
    pub preferred_changeur_id: Option<Uuid>,
    pub max_rate: Option<Decimal>,
    pub min_rating: Option<Decimal>,
    pub location_radius_km: Option<f64>,
    pub client_latitude: Option<f64>,
    pub client_longitude: Option<f64>,
    pub urgency_level: UrgencyLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UrgencyLevel {
    Low,    // Can wait for better rates
    Normal, // Standard matching
    High,   // Need immediate match
    Critical, // Emergency, match at any cost
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub changeur_id: Uuid,
    pub changeur_name: String,
    pub rate: ExchangeRate,
    pub score: Decimal,
    pub estimated_completion_time: Duration,
    pub distance_km: Option<f64>,
    pub availability_status: AvailabilityStatus,
    pub reservation_expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AvailabilityStatus {
    Available,
    Busy,
    LimitedCapacity,
    Offline,
}

#[derive(Debug, Clone)]
pub struct ChangeurScore {
    pub changeur_id: Uuid,
    pub rate_score: Decimal,      // 0-100
    pub rating_score: Decimal,    // 0-100
    pub volume_score: Decimal,    // 0-100
    pub proximity_score: Decimal, // 0-100
    pub availability_score: Decimal, // 0-100
    pub reliability_score: Decimal,  // 0-100
    pub total_score: Decimal,
}

pub struct MatchingEngine {
    reservation_duration: Duration,
    max_concurrent_reservations: usize,
    active_reservations: HashMap<Uuid, Vec<Reservation>>,
}

#[derive(Debug, Clone)]
struct Reservation {
    transaction_id: Uuid,
    changeur_id: Uuid,
    amount: Decimal,
    currency: String,
    expires_at: DateTime<Utc>,
}

impl MatchingEngine {
    pub fn new() -> Self {
        Self {
            reservation_duration: Duration::minutes(15),
            max_concurrent_reservations: 3,
            active_reservations: HashMap::new(),
        }
    }

    pub async fn find_best_match(
        &self,
        criteria: &MatchingCriteria,
        available_rates: Vec<ExchangeRate>,
        changeur_profiles: Vec<ChangeurProfile>,
    ) -> Result<MatchResult, AppError> {
        if available_rates.is_empty() {
            return Err(AppError::not_found("No changeurs available for this currency pair"));
        }

        // Create changeur profile map
        let profile_map: HashMap<Uuid, ChangeurProfile> = changeur_profiles
            .into_iter()
            .map(|p| (p.user_id, p))
            .collect();

        // Score each changeur
        let mut scores = vec![];
        for rate in &available_rates {
            if let Some(profile) = profile_map.get(&rate.changeur_id) {
                let score = self.calculate_changeur_score(
                    rate,
                    profile,
                    criteria,
                )?;
                scores.push((rate.clone(), profile.clone(), score));
            }
        }

        // Sort by total score
        scores.sort_by(|a, b| b.2.total_score.cmp(&a.2.total_score));

        // Apply urgency-based selection
        let selected = match criteria.urgency_level {
            UrgencyLevel::Critical => {
                // Take first available regardless of score
                scores.first()
            }
            UrgencyLevel::High => {
                // Take from top 3
                scores.iter().take(3).max_by_key(|s| s.2.total_score)
            }
            UrgencyLevel::Normal => {
                // Take best score if above threshold
                scores.iter()
                    .find(|s| s.2.total_score >= Decimal::from(60))
                    .or_else(|| scores.first())
            }
            UrgencyLevel::Low => {
                // Only take if excellent score
                scores.iter()
                    .find(|s| s.2.total_score >= Decimal::from(80))
            }
        };

        let (rate, profile, score) = selected
            .ok_or_else(|| AppError::not_found("No suitable changeur found"))?;

        // Check availability
        let availability = self.check_changeur_availability(
            profile.user_id,
            criteria.amount,
            &rate.from_currency,
        )?;

        // Calculate estimated completion time
        let completion_time = self.estimate_completion_time(
            &availability,
            &criteria.urgency_level,
        );

        // Calculate distance if location provided
        let distance = if let (Some(client_lat), Some(client_lon), Some(ch_lat), Some(ch_lon)) = (
            criteria.client_latitude,
            criteria.client_longitude,
            profile.location_latitude,
            profile.location_longitude,
        ) {
            Some(self.calculate_distance(
                client_lat as f64,
                client_lon as f64,
                ch_lat.to_f64().unwrap_or(0.0),
                ch_lon.to_f64().unwrap_or(0.0),
            ))
        } else {
            None
        };

        let reservation_expires = Utc::now() + self.reservation_duration;

        Ok(MatchResult {
            changeur_id: profile.user_id,
            changeur_name: profile.business_name.clone()
                .unwrap_or_else(|| format!("Changeur #{}", profile.user_id)),
            rate: rate.clone(),
            score: score.total_score,
            estimated_completion_time: completion_time,
            distance_km: distance,
            availability_status: availability,
            reservation_expires_at: reservation_expires,
        })
    }

    fn calculate_changeur_score(
        &self,
        rate: &ExchangeRate,
        profile: &ChangeurProfile,
        criteria: &MatchingCriteria,
    ) -> Result<ChangeurScore, AppError> {
        // Rate score (lower is better for customer)
        let rate_score = if rate.sell_rate > Decimal::ZERO {
            let max_acceptable = criteria.max_rate.unwrap_or(rate.sell_rate * Decimal::new(11, 1)); // 110%
            if rate.sell_rate <= max_acceptable {
                ((max_acceptable - rate.sell_rate) / max_acceptable * Decimal::from(100))
                    .min(Decimal::from(100))
                    .max(Decimal::ZERO)
            } else {
                Decimal::ZERO
            }
        } else {
            Decimal::ZERO
        };

        // Rating score
        let rating_score = if profile.rating > Decimal::ZERO {
            (profile.rating / Decimal::from(5) * Decimal::from(100))
                .min(Decimal::from(100))
        } else {
            Decimal::from(50) // Default for new changeurs
        };

        // Volume score (experience)
        let volume_score = if profile.total_volume > Decimal::ZERO {
            ((profile.total_volume / Decimal::from(100000000)).min(Decimal::from(1)) * Decimal::from(100))
                .round_dp(2)
        } else {
            Decimal::from(20)
        };

        // Proximity score
        let proximity_score = if let (Some(client_lat), Some(client_lon), Some(ch_lat), Some(ch_lon)) = (
            criteria.client_latitude,
            criteria.client_longitude,
            profile.location_latitude,
            profile.location_longitude,
        ) {
            let distance = self.calculate_distance(
                client_lat,
                client_lon,
                ch_lat.to_f64().unwrap_or(0.0),
                ch_lon.to_f64().unwrap_or(0.0),
            );
            
            let max_radius = criteria.location_radius_km.unwrap_or(10.0);
            if distance <= max_radius {
                Decimal::from(100) - (Decimal::from(distance as i64) / Decimal::from(max_radius as i64) * Decimal::from(100))
            } else {
                Decimal::ZERO
            }
        } else {
            Decimal::from(50) // Neutral if no location
        };

        // Availability score
        let availability_score = if let Some(available) = rate.available_amount {
            if available >= criteria.amount {
                Decimal::from(100)
            } else {
                (available / criteria.amount * Decimal::from(100)).round_dp(2)
            }
        } else {
            Decimal::from(80) // Assume available if not specified
        };

        // Reliability score (based on completed transactions)
        let reliability_score = if profile.total_transactions > 0 {
            let completion_rate = Decimal::from(95); // TODO: Calculate from actual data
            completion_rate
        } else {
            Decimal::from(50)
        };

        // Calculate weighted total
        let weights = MatchingWeights::default();
        let total_score = (
            rate_score * weights.rate +
            rating_score * weights.rating +
            volume_score * weights.volume +
            proximity_score * weights.proximity +
            availability_score * weights.availability +
            reliability_score * weights.reliability
        ) / weights.total();

        Ok(ChangeurScore {
            changeur_id: profile.user_id,
            rate_score,
            rating_score,
            volume_score,
            proximity_score,
            availability_score,
            reliability_score,
            total_score: total_score.round_dp(2),
        })
    }

    fn check_changeur_availability(
        &self,
        changeur_id: Uuid,
        amount: Decimal,
        currency: &str,
    ) -> Result<AvailabilityStatus, AppError> {
        // Check active reservations
        if let Some(reservations) = self.active_reservations.get(&changeur_id) {
            let active_count = reservations.iter()
                .filter(|r| r.expires_at > Utc::now())
                .count();

            if active_count >= self.max_concurrent_reservations {
                return Ok(AvailabilityStatus::Busy);
            }

            let reserved_amount: Decimal = reservations.iter()
                .filter(|r| r.expires_at > Utc::now() && r.currency == currency)
                .map(|r| r.amount)
                .sum();

            // TODO: Check against actual changeur capacity
            let capacity = Decimal::from(10000000); // 10M XOF
            if reserved_amount + amount > capacity {
                return Ok(AvailabilityStatus::LimitedCapacity);
            }
        }

        Ok(AvailabilityStatus::Available)
    }

    pub fn reserve_changeur(
        &mut self,
        transaction_id: Uuid,
        changeur_id: Uuid,
        amount: Decimal,
        currency: String,
    ) -> Result<DateTime<Utc>, AppError> {
        let expires_at = Utc::now() + self.reservation_duration;
        
        let reservation = Reservation {
            transaction_id,
            changeur_id,
            amount,
            currency,
            expires_at,
        };

        self.active_reservations
            .entry(changeur_id)
            .or_insert_with(Vec::new)
            .push(reservation);

        // Clean expired reservations
        self.clean_expired_reservations();

        info!(
            transaction_id = %transaction_id,
            changeur_id = %changeur_id,
            amount = %amount,
            "Changeur reserved until {:?}",
            expires_at
        );

        Ok(expires_at)
    }

    pub fn release_reservation(&mut self, transaction_id: Uuid) {
        for reservations in self.active_reservations.values_mut() {
            reservations.retain(|r| r.transaction_id != transaction_id);
        }

        info!(transaction_id = %transaction_id, "Reservation released");
    }

    fn clean_expired_reservations(&mut self) {
        let now = Utc::now();
        for reservations in self.active_reservations.values_mut() {
            reservations.retain(|r| r.expires_at > now);
        }

        // Remove changeurs with no reservations
        self.active_reservations.retain(|_, v| !v.is_empty());
    }

    fn estimate_completion_time(
        &self,
        availability: &AvailabilityStatus,
        urgency: &UrgencyLevel,
    ) -> Duration {
        let base_time = match availability {
            AvailabilityStatus::Available => Duration::minutes(5),
            AvailabilityStatus::LimitedCapacity => Duration::minutes(10),
            AvailabilityStatus::Busy => Duration::minutes(20),
            AvailabilityStatus::Offline => Duration::hours(1),
        };

        match urgency {
            UrgencyLevel::Critical => base_time,
            UrgencyLevel::High => base_time + Duration::minutes(2),
            UrgencyLevel::Normal => base_time + Duration::minutes(5),
            UrgencyLevel::Low => base_time + Duration::minutes(10),
        }
    }

    fn calculate_distance(&self, lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        // Haversine formula
        const R: f64 = 6371.0; // Earth radius in km
        
        let dlat = (lat2 - lat1).to_radians();
        let dlon = (lon2 - lon1).to_radians();
        
        let a = (dlat / 2.0).sin().powi(2) +
            lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
        
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        
        R * c
    }

    pub fn rebalance_load(&mut self, changeur_loads: HashMap<Uuid, usize>) {
        // Implement load balancing to distribute transactions evenly
        debug!("Rebalancing load across {} changeurs", changeur_loads.len());
        
        // TODO: Implement sophisticated load balancing algorithm
        // For now, just log the current loads
        for (changeur_id, load) in changeur_loads {
            debug!("Changeur {} has {} active transactions", changeur_id, load);
        }
    }
}

#[derive(Debug, Clone)]
struct MatchingWeights {
    rate: Decimal,
    rating: Decimal,
    volume: Decimal,
    proximity: Decimal,
    availability: Decimal,
    reliability: Decimal,
}

impl Default for MatchingWeights {
    fn default() -> Self {
        Self {
            rate: Decimal::from(30),
            rating: Decimal::from(20),
            volume: Decimal::from(10),
            proximity: Decimal::from(15),
            availability: Decimal::from(15),
            reliability: Decimal::from(10),
        }
    }
}

impl MatchingWeights {
    fn total(&self) -> Decimal {
        self.rate + self.rating + self.volume + 
        self.proximity + self.availability + self.reliability
    }
}