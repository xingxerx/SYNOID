// SYNOID Token Optimizer
// Copyright (c) 2026 Xing_The_Creator | SYNOID

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy)]
pub struct ProviderLimits {
    pub daily_tokens: u64,
    pub daily_requests: u32,
    pub rpm: u32,
}

#[derive(Debug, Clone)]
pub struct ProviderUsage {
    pub tokens_used_today: u64,
    pub requests_today: u32,
    pub requests_this_minute: u32,
    pub last_request_minute: u64,
    pub last_reset_day: u64,
}

impl Default for ProviderUsage {
    fn default() -> Self {
        Self {
            tokens_used_today: 0,
            requests_today: 0,
            requests_this_minute: 0,
            last_request_minute: 0,
            last_reset_day: 0,
        }
    }
}

pub struct TokenOptimizer {
    limits: HashMap<String, ProviderLimits>,
    usage: Arc<Mutex<HashMap<String, ProviderUsage>>>,
}

impl TokenOptimizer {
    pub fn new() -> Self {
        let mut limits = HashMap::new();
        // Groq: 100K tokens/day, 30 rpm, 1000 req/day
        limits.insert(
            "groq".to_string(),
            ProviderLimits {
                daily_tokens: 100_000,
                daily_requests: 1_000,
                rpm: 30,
            },
        );
        // Google AI Studio: 1M tokens/day, 15 rpm, 1500 req/day
        limits.insert(
            "google".to_string(),
            ProviderLimits {
                daily_tokens: 1_000_000,
                daily_requests: 1_500,
                rpm: 15,
            },
        );

        Self {
            limits,
            usage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn current_time() -> (u64, u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let current_minute = now / 60;
        let current_day = now / 86400;
        (current_minute, current_day)
    }

    pub fn can_make_request(&self, provider: &str) -> bool {
        let limits = match self.limits.get(provider) {
            Some(l) => l,
            None => return true, // No limits defined for this provider
        };

        let mut usage_map = self.usage.lock().unwrap();
        let usage = usage_map.entry(provider.to_string()).or_default();
        let (current_minute, current_day) = Self::current_time();

        // Reset counters if needed
        if usage.last_reset_day != current_day {
            usage.tokens_used_today = 0;
            usage.requests_today = 0;
            usage.last_reset_day = current_day;
        }
        if usage.last_request_minute != current_minute {
            usage.requests_this_minute = 0;
            usage.last_request_minute = current_minute;
        }

        // Check limits
        let is_over_limit = usage.requests_this_minute >= limits.rpm
            || usage.requests_today >= limits.daily_requests
            || usage.tokens_used_today >= limits.daily_tokens;

        if is_over_limit {
            warn!(
                "[TOKEN_OPT] Provider {} has reached its rate or daily limit.",
                provider
            );
        } else if usage.tokens_used_today as f64 >= limits.daily_tokens as f64 * 0.8 {
            warn!(
                "[TOKEN_OPT] Provider {} is at >= 80% daily token limit.",
                provider
            );
        }

        !is_over_limit
    }

    pub fn record_usage(&self, provider: &str, estimated_tokens: u64) {
        let mut usage_map = self.usage.lock().unwrap();
        let usage = usage_map.entry(provider.to_string()).or_default();
        let (current_minute, current_day) = Self::current_time();

        if usage.last_reset_day != current_day {
            usage.tokens_used_today = 0;
            usage.requests_today = 0;
            usage.last_reset_day = current_day;
        }
        if usage.last_request_minute != current_minute {
            usage.requests_this_minute = 0;
            usage.last_request_minute = current_minute;
        }

        usage.requests_this_minute += 1;
        usage.requests_today += 1;
        usage.tokens_used_today += estimated_tokens;

        info!(
            "[TOKEN_OPT] Provider {} used {} tokens. Today: {}, Minute: {}/{}",
            provider,
            estimated_tokens,
            usage.tokens_used_today,
            usage.requests_this_minute,
            self.limits.get(provider).map(|l| l.rpm).unwrap_or(0)
        );
    }
}

impl Default for TokenOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
