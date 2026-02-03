//! Init and Upgrade Service
//!
//! Handles canister initialization and upgrade logic.

use crate::state::{init_state, try_get_settings};
use crate::InitArgs;
use ic_siwa::Settings;

/// Initialize the canister with provided arguments
pub fn init(args: InitArgs) {
    let settings = Settings {
        domain: args.domain,
        uri: args.uri,
        salt: args.salt,
        chain_id: args.chain_id,
        session_expiration_time: args.session_expiration_time,
        login_expiration_time: 5 * 60 * 1_000_000_000, // 5 minutes default
        allowed_domains: args.allowed_domains.unwrap_or_default(),
        allowed_canisters: args.allowed_canisters.unwrap_or_default(),
    };

    init_state(settings);
}

/// Handle post-upgrade logic
pub fn post_upgrade(args: Option<InitArgs>) {
    // If new args provided, reinitialize settings
    // Otherwise, state should be preserved from stable memory
    if let Some(args) = args {
        init(args);
    } else if try_get_settings().is_none() {
        // If no settings exist and no args provided, panic
        ic_cdk::trap("Canister upgraded without settings and no InitArgs provided");
    }
}
