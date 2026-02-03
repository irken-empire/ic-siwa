//! Custom getrandom implementation for Internet Computer
//!
//! The IC doesn't have a direct random source available synchronously,
//! so we use a deterministic fallback based on time and caller principal.
//! For cryptographic operations that need true randomness, use the async
//! ic_cdk::api::management_canister::main::raw_rand() instead.

use getrandom::register_custom_getrandom;
use getrandom::Error;

/// Custom getrandom implementation for IC canisters
///
/// This is a fallback for libraries that require getrandom but don't
/// support async. It uses a combination of canister time and a counter
/// to provide pseudo-random bytes.
///
/// WARNING: This is NOT cryptographically secure on its own.
/// For security-critical operations, use raw_rand() from the management canister.
fn ic_getrandom(dest: &mut [u8]) -> Result<(), Error> {
    // Use IC time as entropy source
    let time = ic_cdk::api::time();

    // Simple pseudo-random fill based on time
    // This is deterministic but changes each call due to time advancement
    let mut state = time;
    for byte in dest.iter_mut() {
        // Simple LCG-like mixing
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        *byte = (state >> 33) as u8;
    }

    Ok(())
}

register_custom_getrandom!(ic_getrandom);
