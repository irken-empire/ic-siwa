//! Init and Upgrade Service
//!
//! Handles canister initialization and upgrade logic.
//!
//! Settings and identity mappings are stored in stable memory and persist
//! across upgrades. Transient state (sessions, rate limiter, signature map)
//! is reset on every upgrade.

use crate::state::{has_settings, init_state, init_transient_state};
use crate::InitArgs;
use ic_siwa::{RateLimitSettings, Settings};

/// Initialize the canister with provided arguments
pub fn init(args: InitArgs) {
    let rate_limits = args
        .rate_limits
        .map(|rl| RateLimitSettings {
            max_logins_per_address: rl.max_logins_per_address,
            max_logins_total: rl.max_logins_total,
            window_seconds: rl.window_seconds,
        })
        .unwrap_or_default();

    let settings = Settings {
        domain: args.domain,
        uri: args.uri,
        salt: args.salt,
        chain_id: args.chain_id,
        session_expiration_time: args.session_expiration_time,
        login_expiration_time: args.login_expiration_time.unwrap_or(5 * 60 * 1_000_000_000),
        allowed_domains: args.allowed_domains.unwrap_or_default(),
        allowed_canisters: args.allowed_canisters.unwrap_or_default(),
        delegation_targets: args.delegation_targets.unwrap_or_default(),
        rate_limits,
        debug: args.debug.unwrap_or(false),
    };

    // Validate settings before storing
    if let Err(e) = settings.validate() {
        ic_cdk::trap(format!("Invalid settings: {}", e));
    }

    // Warn about empty ACLs in non-debug mode (security risk in production)
    if !settings.debug {
        if settings.allowed_canisters.is_empty() {
            ic_cdk::println!(
                "WARNING: allowed_canisters is empty — any canister can call this provider"
            );
        }
        if settings.delegation_targets.is_empty() {
            ic_cdk::println!(
                "WARNING: delegation_targets is empty — delegations are valid for any canister"
            );
        }
    }

    if let Err(e) = init_state(settings) {
        ic_cdk::trap(format!("Failed to initialize state: {e}"));
    }
}

/// Handle post-upgrade logic
///
/// If new InitArgs are provided, settings are updated in stable memory.
/// If no args are provided, settings are read from stable memory (they persist).
/// In both cases, transient state (sessions, etc.) is reset.
pub fn post_upgrade(args: Option<InitArgs>) {
    if let Some(args) = args {
        // New args provided: update settings in stable memory and reset transient state
        init(args);
    } else if has_settings() {
        // No args but settings exist in stable memory: just reset transient state
        init_transient_state();
    } else {
        // No args and no settings in stable memory: cannot proceed
        ic_cdk::trap("Canister upgraded without settings and no InitArgs provided");
    }

    ic_cdk::println!(
        "post_upgrade: transient state reset - rate limiter, sessions, and signature map cleared"
    );
}
