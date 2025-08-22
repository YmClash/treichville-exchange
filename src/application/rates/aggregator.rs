use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    domain::rate::{ExchangeRate, PublicRate, RateQuote},
    shared::errors::AppError,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedRate {
    pub from_currency: String,
    pub to_currency: String,
    pub best_buy_rate: Decimal,
    pub best_sell_rate: Decimal,
    pub worst_buy_rate: Decimal,
    pub worst_sell_rate: Decimal,
    pub average_buy_rate: Decimal,
    pub average_sell_rate: Decimal,
    pub median_buy_rate: Decimal,
    pub median_sell_rate: Decimal,
    pub spread_average: Decimal,
    pub liquidity_total: Decimal,
    pub changeurs_count: usize,
    pub confidence_score: Decimal, // 0-100
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDepth {
    pub currency_pair: String,
    pub bids: Vec<OrderBookEntry>, // Buy orders
    pub asks: Vec<OrderBookEntry>, // Sell orders
    pub spread: Decimal,
    pub mid_price: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookEntry {
    pub price: Decimal,
    pub amount: Decimal,
    pub changeur_id: Uuid,
    pub changeur_name: String,
}

pub struct RateAggregator;

impl RateAggregator {
    pub fn aggregate_rates(rates: &[ExchangeRate]) -> Result<AggregatedRate, AppError> {
        if rates.is_empty() {
            return Err(AppError::not_found("No rates available"));
        }

        let first = &rates[0];
        let mut buy_rates: Vec<Decimal> = rates.iter().map(|r| r.buy_rate).collect();
        let mut sell_rates: Vec<Decimal> = rates.iter().map(|r| r.sell_rate).collect();
        
        buy_rates.sort();
        sell_rates.sort();

        let best_buy_rate = *buy_rates.last().unwrap(); // Higher is better for buying
        let worst_buy_rate = *buy_rates.first().unwrap();
        let best_sell_rate = *sell_rates.first().unwrap(); // Lower is better for selling
        let worst_sell_rate = *sell_rates.last().unwrap();

        let average_buy_rate = buy_rates.iter().sum::<Decimal>() / Decimal::from(buy_rates.len());
        let average_sell_rate = sell_rates.iter().sum::<Decimal>() / Decimal::from(sell_rates.len());

        let median_buy_rate = Self::calculate_median(&buy_rates);
        let median_sell_rate = Self::calculate_median(&sell_rates);

        let spread_average = rates.iter()
            .map(|r| r.spread)
            .sum::<Decimal>() / Decimal::from(rates.len());

        let liquidity_total = rates.iter()
            .filter_map(|r| r.available_amount)
            .sum::<Decimal>();

        let confidence_score = Self::calculate_confidence_score(rates);

        Ok(AggregatedRate {
            from_currency: first.from_currency.clone(),
            to_currency: first.to_currency.clone(),
            best_buy_rate,
            best_sell_rate,
            worst_buy_rate,
            worst_sell_rate,
            average_buy_rate,
            average_sell_rate,
            median_buy_rate,
            median_sell_rate,
            spread_average,
            liquidity_total,
            changeurs_count: rates.len(),
            confidence_score,
            last_updated: Utc::now(),
        })
    }

    pub fn calculate_median(sorted_values: &[Decimal]) -> Decimal {
        let len = sorted_values.len();
        if len % 2 == 0 {
            (sorted_values[len / 2 - 1] + sorted_values[len / 2]) / Decimal::from(2)
        } else {
            sorted_values[len / 2]
        }
    }

    pub fn calculate_confidence_score(rates: &[ExchangeRate]) -> Decimal {
        let mut score = Decimal::from(100);

        // Reduce score if few changeurs
        if rates.len() < 3 {
            score -= Decimal::from(30);
        } else if rates.len() < 5 {
            score -= Decimal::from(15);
        }

        // Calculate standard deviation
        let buy_rates: Vec<Decimal> = rates.iter().map(|r| r.buy_rate).collect();
        let sell_rates: Vec<Decimal> = rates.iter().map(|r| r.sell_rate).collect();
        
        let buy_std_dev = Self::calculate_std_deviation(&buy_rates);
        let sell_std_dev = Self::calculate_std_deviation(&sell_rates);

        // High deviation reduces confidence
        let avg_buy = buy_rates.iter().sum::<Decimal>() / Decimal::from(buy_rates.len());
        let avg_sell = sell_rates.iter().sum::<Decimal>() / Decimal::from(sell_rates.len());

        if avg_buy > Decimal::ZERO {
            let buy_cv = (buy_std_dev / avg_buy * Decimal::from(100)).abs();
            if buy_cv > Decimal::from(10) {
                score -= buy_cv.min(Decimal::from(30));
            }
        }

        if avg_sell > Decimal::ZERO {
            let sell_cv = (sell_std_dev / avg_sell * Decimal::from(100)).abs();
            if sell_cv > Decimal::from(10) {
                score -= sell_cv.min(Decimal::from(30));
            }
        }

        // Check for stale rates
        let now = Utc::now();
        let stale_count = rates.iter()
            .filter(|r| r.updated_at.map_or(true, |updated| (now - updated).num_hours() > 1))
            .count();
        
        if stale_count > rates.len() / 2 {
            score -= Decimal::from(20);
        }

        score.max(Decimal::ZERO).min(Decimal::from(100))
    }

    pub fn calculate_std_deviation(values: &[Decimal]) -> Decimal {
        if values.is_empty() {
            return Decimal::ZERO;
        }

        let mean = values.iter().sum::<Decimal>() / Decimal::from(values.len());
        let variance = values.iter()
            .map(|v| (*v - mean).powi(2))
            .sum::<Decimal>() / Decimal::from(values.len());

        // Simple square root approximation for Decimal
        Self::decimal_sqrt(variance)
    }

    fn decimal_sqrt(value: Decimal) -> Decimal {
        if value <= Decimal::ZERO {
            return Decimal::ZERO;
        }

        let mut x = value;
        let mut last_x = Decimal::ZERO;
        let epsilon = Decimal::new(1, 10); // 0.0000000001

        while (x - last_x).abs() > epsilon {
            last_x = x;
            x = (x + value / x) / Decimal::from(2);
        }

        x
    }

    pub fn detect_outliers(rates: &[ExchangeRate]) -> Vec<Uuid> {
        if rates.len() < 3 {
            return vec![];
        }

        let mut outliers = vec![];
        
        let buy_rates: Vec<Decimal> = rates.iter().map(|r| r.buy_rate).collect();
        let sell_rates: Vec<Decimal> = rates.iter().map(|r| r.sell_rate).collect();
        
        let buy_mean = buy_rates.iter().sum::<Decimal>() / Decimal::from(buy_rates.len());
        let sell_mean = sell_rates.iter().sum::<Decimal>() / Decimal::from(sell_rates.len());
        
        let buy_std_dev = Self::calculate_std_deviation(&buy_rates);
        let sell_std_dev = Self::calculate_std_deviation(&sell_rates);

        // Z-score method: outlier if |z| > 2.5
        let z_threshold = dec!(2.5);

        for rate in rates {
            if buy_std_dev > Decimal::ZERO {
                let buy_z = ((rate.buy_rate - buy_mean) / buy_std_dev).abs();
                if buy_z > z_threshold {
                    outliers.push(rate.changeur_id);
                    continue;
                }
            }

            if sell_std_dev > Decimal::ZERO {
                let sell_z = ((rate.sell_rate - sell_mean) / sell_std_dev).abs();
                if sell_z > z_threshold {
                    outliers.push(rate.changeur_id);
                }
            }
        }

        outliers
    }

    pub fn build_market_depth(
        rates: &[ExchangeRate],
        changeur_names: HashMap<Uuid, String>,
    ) -> MarketDepth {
        let mut bids = vec![];
        let mut asks = vec![];

        for rate in rates {
            if let Some(amount) = rate.available_amount {
                let changeur_name = changeur_names
                    .get(&rate.changeur_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());

                // Buy orders (bids)
                bids.push(OrderBookEntry {
                    price: rate.buy_rate,
                    amount,
                    changeur_id: rate.changeur_id,
                    changeur_name: changeur_name.clone(),
                });

                // Sell orders (asks)
                asks.push(OrderBookEntry {
                    price: rate.sell_rate,
                    amount,
                    changeur_id: rate.changeur_id,
                    changeur_name,
                });
            }
        }

        // Sort bids descending (best price first)
        bids.sort_by(|a, b| b.price.cmp(&a.price));
        // Sort asks ascending (best price first)
        asks.sort_by(|a, b| a.price.cmp(&b.price));

        let best_bid = bids.first().map(|b| b.price).unwrap_or(Decimal::ZERO);
        let best_ask = asks.first().map(|a| a.price).unwrap_or(Decimal::ZERO);
        let spread = if best_bid > Decimal::ZERO && best_ask > Decimal::ZERO {
            best_ask - best_bid
        } else {
            Decimal::ZERO
        };
        let mid_price = if best_bid > Decimal::ZERO && best_ask > Decimal::ZERO {
            (best_bid + best_ask) / Decimal::from(2)
        } else {
            Decimal::ZERO
        };

        let currency_pair = if !rates.is_empty() {
            format!("{}/{}", rates[0].from_currency, rates[0].to_currency)
        } else {
            "N/A".to_string()
        };

        MarketDepth {
            currency_pair,
            bids,
            asks,
            spread,
            mid_price,
            timestamp: Utc::now(),
        }
    }

    pub fn calculate_savings(
        best_rate: Decimal,
        average_rate: Decimal,
        amount: Decimal,
        is_buying: bool,
    ) -> Decimal {
        if is_buying {
            // For buying, lower rate is better
            (average_rate - best_rate) * amount
        } else {
            // For selling, higher rate is better
            (best_rate - average_rate) * amount
        }
    }

    pub fn rank_changeurs_by_competitiveness(rates: &[ExchangeRate]) -> Vec<ChangeurRanking> {
        let mut rankings = vec![];
        
        let avg_buy = rates.iter().map(|r| r.buy_rate).sum::<Decimal>() / Decimal::from(rates.len());
        let avg_sell = rates.iter().map(|r| r.sell_rate).sum::<Decimal>() / Decimal::from(rates.len());

        for rate in rates {
            // Score based on deviation from average (better rates get higher scores)
            let buy_score = ((rate.buy_rate - avg_buy) / avg_buy * Decimal::from(100)).round_dp(2);
            let sell_score = ((avg_sell - rate.sell_rate) / avg_sell * Decimal::from(100)).round_dp(2);
            let spread_score = (Decimal::from(100) - (rate.spread * Decimal::from(100))).round_dp(2);
            
            let liquidity_score = if let Some(amount) = rate.available_amount {
                (amount / Decimal::from(1000000)).min(Decimal::from(100)) // Score based on millions available
            } else {
                Decimal::from(50) // Default score if no amount specified
            };

            let total_score = (buy_score + sell_score + spread_score + liquidity_score) / Decimal::from(4);

            rankings.push(ChangeurRanking {
                changeur_id: rate.changeur_id,
                buy_rate: rate.buy_rate,
                sell_rate: rate.sell_rate,
                spread: rate.spread,
                available_amount: rate.available_amount,
                competitiveness_score: total_score.round_dp(2),
            });
        }

        rankings.sort_by(|a, b| b.competitiveness_score.cmp(&a.competitiveness_score));
        rankings
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeurRanking {
    pub changeur_id: Uuid,
    pub buy_rate: Decimal,
    pub sell_rate: Decimal,
    pub spread: Decimal,
    pub available_amount: Option<Decimal>,
    pub competitiveness_score: Decimal,
}