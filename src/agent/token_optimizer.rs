<<<<<<< HEAD
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
=======
// SYNOID MCP Token Optimizer
// Copyright (c) 2026 Xing_The_Creator | SYNOID
//
// Tracks token usage across free-tier LLM providers and enforces budgets
// so SYNOID never burns through the free limits.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

/// Per-provider rate limits and token budgets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderBudget {
    /// Human-readable provider name.
    pub name: String,
    /// Maximum tokens per day (0 = unlimited).
    pub daily_token_limit: u64,
    /// Maximum requests per minute.
    pub rpm_limit: u32,
    /// Maximum requests per day (0 = unlimited).
    pub daily_request_limit: u64,
    /// Tokens consumed today.
    pub tokens_used_today: u64,
    /// Requests made today.
    pub requests_today: u64,
    /// Requests made in the current minute window.
    pub requests_this_minute: u32,
    /// Timestamp of current day (resets at midnight UTC).
    pub day_start: u64,
    /// Timestamp of current minute window.
    pub minute_start: u64,
}

impl ProviderBudget {
    pub fn new(
        name: &str,
        daily_token_limit: u64,
        rpm_limit: u32,
        daily_request_limit: u64,
    ) -> Self {
        let now = current_timestamp();
        Self {
            name: name.to_string(),
            daily_token_limit,
            rpm_limit,
            daily_request_limit,
            tokens_used_today: 0,
            requests_today: 0,
            requests_this_minute: 0,
            day_start: now - (now % 86400),
            minute_start: now - (now % 60),
        }
    }

    /// Check if we can make another request without exceeding limits.
    pub fn can_request(&mut self) -> bool {
        self.maybe_reset_windows();

        if self.daily_request_limit > 0 && self.requests_today >= self.daily_request_limit {
            return false;
        }
        if self.requests_this_minute >= self.rpm_limit {
            return false;
        }
        true
    }

    /// Check if we have token budget remaining.
    pub fn has_token_budget(&mut self, estimated_tokens: u64) -> bool {
        self.maybe_reset_windows();

        if self.daily_token_limit == 0 {
            return true; // Unlimited
        }
        self.tokens_used_today + estimated_tokens <= self.daily_token_limit
    }

    /// Record a completed request.
    pub fn record_usage(&mut self, tokens_used: u64) {
        self.maybe_reset_windows();
        self.tokens_used_today += tokens_used;
        self.requests_today += 1;
        self.requests_this_minute += 1;
    }

    /// Reset counters when time windows roll over.
    fn maybe_reset_windows(&mut self) {
        let now = current_timestamp();
        let today = now - (now % 86400);
        let this_minute = now - (now % 60);

        if today != self.day_start {
            self.tokens_used_today = 0;
            self.requests_today = 0;
            self.day_start = today;
        }
        if this_minute != self.minute_start {
            self.requests_this_minute = 0;
            self.minute_start = this_minute;
        }
    }

    /// Percentage of daily token budget consumed.
    pub fn usage_percent(&self) -> f64 {
        if self.daily_token_limit == 0 {
            return 0.0;
        }
        (self.tokens_used_today as f64 / self.daily_token_limit as f64) * 100.0
    }
}

/// The central token optimizer that manages all provider budgets.
pub struct TokenOptimizer {
    budgets: Arc<Mutex<HashMap<String, ProviderBudget>>>,
}

impl TokenOptimizer {
    pub fn new() -> Self {
        Self {
            budgets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a provider with its rate limits.
    pub fn register_provider(&self, id: &str, budget: ProviderBudget) {
        let mut budgets = self.budgets.lock().unwrap();
        info!(
            "[TOKEN_OPT] Registered provider '{}': {}rpm, {} tokens/day, {} req/day",
            budget.name, budget.rpm_limit, budget.daily_token_limit, budget.daily_request_limit
        );
        budgets.insert(id.to_string(), budget);
    }

    /// Check if a provider can accept a request.
    pub fn can_use(&self, provider_id: &str, estimated_tokens: u64) -> bool {
        let mut budgets = self.budgets.lock().unwrap();
        if let Some(budget) = budgets.get_mut(provider_id) {
            budget.can_request() && budget.has_token_budget(estimated_tokens)
        } else {
            true // Unknown provider = no limits tracked
        }
    }

    /// Record token usage for a provider after a successful call.
    pub fn record(&self, provider_id: &str, tokens_used: u64) {
        let mut budgets = self.budgets.lock().unwrap();
        if let Some(budget) = budgets.get_mut(provider_id) {
            budget.record_usage(tokens_used);

            let pct = budget.usage_percent();
            if pct > 80.0 {
                warn!(
                    "[TOKEN_OPT] {} at {:.1}% daily token budget ({}/{} tokens)",
                    budget.name, pct, budget.tokens_used_today, budget.daily_token_limit
                );
            }
        }
    }

    /// Pick the best available provider from a priority list.
    /// Returns the first provider that has budget remaining.
    pub fn pick_available(&self, provider_ids: &[&str], estimated_tokens: u64) -> Option<String> {
        let mut budgets = self.budgets.lock().unwrap();
        for id in provider_ids {
            if let Some(budget) = budgets.get_mut(*id) {
                if budget.can_request() && budget.has_token_budget(estimated_tokens) {
                    return Some(id.to_string());
                }
            }
        }
        None
    }

    /// Get a status report of all providers.
    pub fn status_report(&self) -> Vec<(String, f64, u64, u64)> {
        let budgets = self.budgets.lock().unwrap();
        budgets
            .iter()
            .map(|(id, b)| {
                (
                    id.clone(),
                    b.usage_percent(),
                    b.tokens_used_today,
                    b.requests_today,
                )
            })
            .collect()
    }

    /// Human-readable status for logging/GUI.
    pub fn display_status(&self) -> String {
        let budgets = self.budgets.lock().unwrap();
        let mut lines = Vec::new();
        for (id, b) in budgets.iter() {
            lines.push(format!(
                "{}: {:.1}% tokens ({}/{}), {} req today, {}/{} rpm",
                id,
                b.usage_percent(),
                b.tokens_used_today,
                b.daily_token_limit,
                b.requests_today,
                b.requests_this_minute,
                b.rpm_limit
            ));
        }
        if lines.is_empty() {
            "No providers registered".to_string()
        } else {
            lines.join("\n")
        }
    }
}

impl Default for TokenOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a TokenOptimizer pre-configured with Groq + Google AI Studio free-tier limits.
pub fn create_default_optimizer() -> TokenOptimizer {
    let optimizer = TokenOptimizer::new();

    // Groq Free Tier:
    // - Llama 3.3 70B: 1,000 req/day, 30 req/min, ~100k tokens/day
    // - Llama 3.1 8B: 14,400 req/day, 30 req/min
    optimizer.register_provider("groq", ProviderBudget::new("Groq", 100_000, 30, 1_000));

    // Groq fast model (higher request limit for small models)
    optimizer.register_provider(
        "groq_fast",
        ProviderBudget::new("Groq Fast", 500_000, 30, 14_400),
    );

    // Google AI Studio Free Tier:
    // - Gemini 2.0 Flash: 15 req/min, 1M tokens/day, 1500 req/day
    optimizer.register_provider(
        "google_vision",
        ProviderBudget::new("Google AI Studio", 1_000_000, 15, 1_500),
    );

    optimizer
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_tracking() {
        let mut budget = ProviderBudget::new("test", 1000, 5, 100);
        assert!(budget.can_request());
        assert!(budget.has_token_budget(500));

        budget.record_usage(800);
        assert!(budget.has_token_budget(100));
        assert!(!budget.has_token_budget(300));
    }

    #[test]
    fn test_rpm_limit() {
        let mut budget = ProviderBudget::new("test", 0, 2, 0);
        assert!(budget.can_request());
        budget.record_usage(0);
        assert!(budget.can_request());
        budget.record_usage(0);
        assert!(!budget.can_request()); // Hit RPM limit
    }

    #[test]
    fn test_optimizer_pick() {
        let opt = TokenOptimizer::new();
        opt.register_provider("a", ProviderBudget::new("A", 100, 5, 10));
        opt.register_provider("b", ProviderBudget::new("B", 10000, 30, 1000));

        // Exhaust provider A
        for _ in 0..10 {
            opt.record("a", 10);
        }

        // A is exhausted on requests, B should be picked
        let pick = opt.pick_available(&["a", "b"], 100);
        assert_eq!(pick, Some("b".to_string()));
    }
}
>>>>>>> c55b0d9e6ebf2105e2d2c161f2b2839c68f38981
