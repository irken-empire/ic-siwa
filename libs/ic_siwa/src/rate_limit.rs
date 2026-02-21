//! Rate limiting for IC-SIWA
//!
//! Provides protection against cycle drain attacks by limiting
//! the number of login attempts per address and globally.

use crate::error::SiwaError;
use crate::settings::RateLimitSettings;
use std::collections::HashMap;

/// Hard cap on the number of unique addresses tracked in a single window.
/// Prevents unbounded memory growth regardless of rate-limit configuration.
const MAX_PER_ADDRESS_ENTRIES: usize = 10_000;

/// Rate limiter for tracking and enforcing login attempt limits
#[derive(Clone, Debug)]
pub struct RateLimiter {
    /// Settings for rate limiting
    settings: RateLimitSettings,
    /// Per-address attempt counts: address -> (count, window_start_ns)
    per_address: HashMap<String, (u32, u64)>,
    /// Global attempt count and window start
    global: (u32, u64),
}

impl RateLimiter {
    /// Create a new rate limiter with the given settings
    pub fn new(settings: RateLimitSettings) -> Self {
        Self {
            settings,
            per_address: HashMap::new(),
            global: (0, 0),
        }
    }

    /// Create a rate limiter with default (permissive) settings
    pub fn default_permissive() -> Self {
        Self::new(RateLimitSettings {
            max_logins_per_address: u32::MAX,
            max_logins_total: u32::MAX,
            window_seconds: 3600,
        })
    }

    /// Get window duration in nanoseconds
    fn window_ns(&self) -> u64 {
        self.settings.window_seconds.saturating_mul(1_000_000_000)
    }

    /// Check if a timestamp is within the current window.
    /// If the clock jumped backward (now < window_start), treat as a new window.
    fn is_in_window(&self, window_start: u64, now: u64) -> bool {
        if now < window_start {
            // Clock jumped backward (e.g., IC subnet recovery) — reset window
            return false;
        }
        now < window_start.saturating_add(self.window_ns())
    }

    /// Check rate limit and record an attempt for the given address
    ///
    /// # Arguments
    /// * `address` - The Avalanche address attempting to log in
    /// * `now_ns` - Current timestamp in nanoseconds
    ///
    /// # Returns
    /// * `Ok(())` if the attempt is allowed
    /// * `Err(SiwaError::RateLimited)` if rate limit exceeded
    pub fn check_and_record(&mut self, address: &str, now_ns: u64) -> Result<(), SiwaError> {
        let address_lower = address.to_lowercase();

        // Periodically cleanup expired per-address entries to bound memory growth.
        // Runs every 100 unique addresses to amortize the cost.
        if self.per_address.len() > 100 {
            self.cleanup_expired(now_ns);
        }

        // Hard cap: prevent unbounded memory growth regardless of rate-limit config.
        // Already-tracked addresses can still make attempts (subject to per-address limit).
        if self.per_address.len() >= MAX_PER_ADDRESS_ENTRIES
            && !self.per_address.contains_key(&address_lower)
        {
            return Err(SiwaError::RateLimited(
                "Too many unique addresses in current window".to_string(),
            ));
        }

        // Check and update global rate limit
        self.check_global(now_ns)?;

        // Check and update per-address rate limit
        self.check_per_address(&address_lower, now_ns)?;

        // Record the attempt
        self.record_attempt(&address_lower, now_ns);

        Ok(())
    }

    /// Check global rate limit without recording
    fn check_global(&self, now_ns: u64) -> Result<(), SiwaError> {
        let (count, window_start) = self.global;

        // If we're in a new window, the count effectively resets
        if !self.is_in_window(window_start, now_ns) {
            return Ok(());
        }

        if count >= self.settings.max_logins_total {
            return Err(SiwaError::RateLimited(format!(
                "Global rate limit exceeded: {} attempts in {} seconds",
                self.settings.max_logins_total, self.settings.window_seconds
            )));
        }

        Ok(())
    }

    /// Check per-address rate limit without recording
    fn check_per_address(&self, address: &str, now_ns: u64) -> Result<(), SiwaError> {
        if let Some(&(count, window_start)) = self.per_address.get(address) {
            // If we're in a new window, the count effectively resets
            if !self.is_in_window(window_start, now_ns) {
                return Ok(());
            }

            if count >= self.settings.max_logins_per_address {
                return Err(SiwaError::RateLimited(format!(
                    "Address rate limit exceeded: {} attempts in {} seconds for {}",
                    self.settings.max_logins_per_address, self.settings.window_seconds, address
                )));
            }
        }

        Ok(())
    }

    /// Record an attempt (called after checks pass)
    fn record_attempt(&mut self, address: &str, now_ns: u64) {
        // Update global counter
        let (count, window_start) = self.global;
        if self.is_in_window(window_start, now_ns) {
            self.global = (count + 1, window_start);
        } else {
            self.global = (1, now_ns);
        }

        // Update per-address counter
        if let Some(&(count, window_start)) = self.per_address.get(address) {
            if self.is_in_window(window_start, now_ns) {
                self.per_address
                    .insert(address.to_string(), (count + 1, window_start));
            } else {
                self.per_address.insert(address.to_string(), (1, now_ns));
            }
        } else {
            self.per_address.insert(address.to_string(), (1, now_ns));
        }
    }

    /// Clean up expired entries to prevent memory growth
    ///
    /// Call this periodically (e.g., on every Nth request or via timer)
    pub fn cleanup_expired(&mut self, now_ns: u64) {
        // Get window duration to avoid borrowing self in closure
        let window_ns = self.window_ns();

        // Clean up per-address entries (also handles clock backward jumps)
        self.per_address.retain(|_, &mut (_, window_start)| {
            now_ns >= window_start && now_ns < window_start.saturating_add(window_ns)
        });

        // Reset global if window expired or clock jumped backward
        let (_, window_start) = self.global;
        if now_ns < window_start || now_ns >= window_start.saturating_add(window_ns) {
            self.global = (0, 0);
        }
    }

    /// Get current attempt count for an address (for diagnostics)
    pub fn get_address_count(&self, address: &str, now_ns: u64) -> u32 {
        let address_lower = address.to_lowercase();
        if let Some(&(count, window_start)) = self.per_address.get(&address_lower) {
            if self.is_in_window(window_start, now_ns) {
                return count;
            }
        }
        0
    }

    /// Get current global attempt count (for diagnostics)
    pub fn get_global_count(&self, now_ns: u64) -> u32 {
        let (count, window_start) = self.global;
        if self.is_in_window(window_start, now_ns) {
            count
        } else {
            0
        }
    }

    /// Get remaining attempts for an address
    pub fn remaining_for_address(&self, address: &str, now_ns: u64) -> u32 {
        let current = self.get_address_count(address, now_ns);
        self.settings.max_logins_per_address.saturating_sub(current)
    }

    /// Get remaining global attempts
    pub fn remaining_global(&self, now_ns: u64) -> u32 {
        let current = self.get_global_count(now_ns);
        self.settings.max_logins_total.saturating_sub(current)
    }

    /// Update settings (e.g., after canister upgrade)
    pub fn update_settings(&mut self, settings: RateLimitSettings) {
        self.settings = settings;
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(RateLimitSettings::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_settings() -> RateLimitSettings {
        RateLimitSettings {
            max_logins_per_address: 3,
            max_logins_total: 10,
            window_seconds: 60, // 1 minute for easier testing
        }
    }

    #[test]
    fn test_rate_limiter_allows_within_limits() {
        let mut limiter = RateLimiter::new(test_settings());
        let now = 1_000_000_000_000u64; // 1 second in ns

        // Should allow first 3 attempts
        assert!(limiter.check_and_record("0x1234", now).is_ok());
        assert!(limiter.check_and_record("0x1234", now + 1000).is_ok());
        assert!(limiter.check_and_record("0x1234", now + 2000).is_ok());
    }

    #[test]
    fn test_rate_limiter_blocks_per_address() {
        let mut limiter = RateLimiter::new(test_settings());
        let now = 1_000_000_000_000u64;

        // Use up the per-address limit
        for i in 0..3 {
            assert!(limiter.check_and_record("0x1234", now + i * 1000).is_ok());
        }

        // 4th attempt should be blocked
        let result = limiter.check_and_record("0x1234", now + 3000);
        assert!(result.is_err());
        assert!(matches!(result, Err(SiwaError::RateLimited(_))));
    }

    #[test]
    fn test_rate_limiter_different_addresses() {
        let mut limiter = RateLimiter::new(test_settings());
        let now = 1_000_000_000_000u64;

        // Use up limit for address 1
        for i in 0..3 {
            assert!(limiter.check_and_record("0x1111", now + i * 1000).is_ok());
        }

        // Address 2 should still work
        assert!(limiter.check_and_record("0x2222", now + 4000).is_ok());
    }

    #[test]
    fn test_rate_limiter_global_limit() {
        let mut limiter = RateLimiter::new(test_settings());
        let now = 1_000_000_000_000u64;

        // Use up global limit with different addresses
        for i in 0..10 {
            let addr = format!("0x{:04x}", i);
            assert!(limiter
                .check_and_record(&addr, now + i as u64 * 1000)
                .is_ok());
        }

        // 11th attempt should be blocked globally
        let result = limiter.check_and_record("0xnew", now + 11000);
        assert!(result.is_err());
        assert!(matches!(result, Err(SiwaError::RateLimited(_))));
    }

    #[test]
    fn test_rate_limiter_window_reset() {
        let mut limiter = RateLimiter::new(test_settings());
        let now = 1_000_000_000_000u64;
        let window_ns = 60 * 1_000_000_000u64; // 60 seconds in ns

        // Use up the limit
        for i in 0..3 {
            assert!(limiter.check_and_record("0x1234", now + i * 1000).is_ok());
        }

        // Should be blocked within window
        assert!(limiter.check_and_record("0x1234", now + 5000).is_err());

        // Should work after window expires
        let after_window = now + window_ns + 1;
        assert!(limiter.check_and_record("0x1234", after_window).is_ok());
    }

    #[test]
    fn test_rate_limiter_cleanup() {
        let mut limiter = RateLimiter::new(test_settings());
        let now = 1_000_000_000_000u64;
        let window_ns = 60 * 1_000_000_000u64;

        // Add some entries
        limiter.check_and_record("0x1111", now).unwrap();
        limiter.check_and_record("0x2222", now).unwrap();
        assert_eq!(limiter.per_address.len(), 2);

        // Cleanup within window should keep entries
        limiter.cleanup_expired(now + 1000);
        assert_eq!(limiter.per_address.len(), 2);

        // Cleanup after window should remove entries
        limiter.cleanup_expired(now + window_ns + 1);
        assert_eq!(limiter.per_address.len(), 0);
    }

    #[test]
    fn test_rate_limiter_auto_cleanup_expired_entries() {
        let settings = RateLimitSettings {
            max_logins_per_address: 100,
            max_logins_total: 10000,
            window_seconds: 60,
        };
        let mut limiter = RateLimiter::new(settings);
        let now = 1_000_000_000_000u64;
        let window_ns = 60 * 1_000_000_000u64;

        // Fill up 150 unique addresses in the same window
        for i in 0..150u32 {
            let addr = format!("0x{:08x}", i);
            limiter.check_and_record(&addr, now).unwrap();
        }
        assert!(limiter.per_address.len() >= 100);

        // Move past the window and add a new address — triggers auto cleanup
        let after_window = now + window_ns + 1;
        limiter.check_and_record("0xaa00bb", after_window).unwrap();

        // Expired entries should have been cleaned up
        assert_eq!(limiter.per_address.len(), 1);
    }

    #[test]
    fn test_rate_limiter_case_insensitive() {
        let mut limiter = RateLimiter::new(test_settings());
        let now = 1_000_000_000_000u64;

        // Mixed case should be treated the same
        limiter.check_and_record("0xABCD", now).unwrap();
        limiter.check_and_record("0xabcd", now + 1000).unwrap();
        limiter.check_and_record("0xAbCd", now + 2000).unwrap();

        // 4th attempt should be blocked
        assert!(limiter.check_and_record("0xABCD", now + 3000).is_err());
    }

    #[test]
    fn test_rate_limiter_clock_jump_backward() {
        let mut limiter = RateLimiter::new(test_settings());
        let now = 1_000_000_000_000u64;

        // Use up the limit
        for i in 0..3 {
            assert!(limiter.check_and_record("0x1234", now + i * 1000).is_ok());
        }

        // Should be blocked at current time
        assert!(limiter.check_and_record("0x1234", now + 5000).is_err());

        // Clock jumps backward — should reset window and allow attempts again
        let jumped_back = now - 5_000_000_000; // 5 seconds before original start
        assert!(limiter.check_and_record("0x1234", jumped_back).is_ok());
    }

    #[test]
    fn test_rate_limiter_diagnostics() {
        let mut limiter = RateLimiter::new(test_settings());
        let now = 1_000_000_000_000u64;

        assert_eq!(limiter.get_address_count("0x1234", now), 0);
        assert_eq!(limiter.remaining_for_address("0x1234", now), 3);
        assert_eq!(limiter.remaining_global(now), 10);

        limiter.check_and_record("0x1234", now).unwrap();

        assert_eq!(limiter.get_address_count("0x1234", now), 1);
        assert_eq!(limiter.remaining_for_address("0x1234", now), 2);
        assert_eq!(limiter.remaining_global(now), 9);
    }

    #[test]
    fn test_rate_limiter_hard_cap_enforced() {
        let settings = RateLimitSettings {
            max_logins_per_address: u32::MAX,
            max_logins_total: u32::MAX,
            window_seconds: 3600,
        };
        let mut limiter = RateLimiter::new(settings);
        let now = 1_000_000_000_000u64;

        // Fill the map to the hard cap
        for i in 0..MAX_PER_ADDRESS_ENTRIES {
            let addr = format!("0x{:08x}", i);
            assert!(
                limiter.check_and_record(&addr, now + i as u64).is_ok(),
                "address {} should be allowed",
                i
            );
        }

        // A new address beyond the cap should be rejected
        let result =
            limiter.check_and_record("0xnew_address", now + MAX_PER_ADDRESS_ENTRIES as u64);
        assert!(
            matches!(result, Err(SiwaError::RateLimited(_))),
            "expected RateLimited for new address beyond hard cap"
        );

        // An already-tracked address should still succeed
        let result =
            limiter.check_and_record("0x00000000", now + MAX_PER_ADDRESS_ENTRIES as u64 + 1);
        assert!(
            result.is_ok(),
            "already-tracked address should still be allowed"
        );
    }
}
