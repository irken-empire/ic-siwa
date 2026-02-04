//! IC-SIWA Provider Canister
//!
//! This canister provides Sign-In with Avalanche (SIWA) authentication
//! for the Internet Computer. It allows users to authenticate using their
//! Avalanche wallet and receive an ICP principal in return.

use candid::{CandidType, Principal};
use ic_cdk_macros::{init, post_upgrade, query, update};
use serde::Deserialize;

// Register custom getrandom implementation for IC WASM environment
mod random;

mod service;
mod state;

pub use service::siwa_get_delegation::{Delegation, DelegationChain, SignedDelegation};
pub use service::siwa_login::LoginResponse;
pub use service::siwa_prepare_login::PrepareLoginResponse;

/// Rate limit configuration for Candid
#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct RateLimitArgs {
    /// Max prepare_login calls per address per window
    pub max_logins_per_address: u32,
    /// Max total prepare_login calls per window
    pub max_logins_total: u32,
    /// Time window in seconds
    pub window_seconds: u64,
}

/// Canister initialization arguments
#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct InitArgs {
    /// Domain for SIWA messages
    pub domain: String,
    /// URI for SIWA messages
    pub uri: String,
    /// Salt for principal derivation
    pub salt: String,
    /// Chain ID (Avalanche C-Chain)
    pub chain_id: u64,
    /// Session expiration in nanoseconds
    pub session_expiration_time: u64,
    /// Optional list of allowed domains
    pub allowed_domains: Option<Vec<String>>,
    /// Optional list of allowed canister IDs
    pub allowed_canisters: Option<Vec<Principal>>,
    /// Optional delegation targets - canisters that delegations are valid for
    pub delegation_targets: Option<Vec<Principal>>,
    /// Optional rate limiting configuration
    pub rate_limits: Option<RateLimitArgs>,
    /// Enable debug endpoints (should be false in production)
    pub debug: Option<bool>,
}

/// Initialize the canister
#[init]
fn init(args: InitArgs) {
    service::init_upgrade::init(args);
    ic_cdk::println!("ic_siwa_provider initialized");
}

/// Post-upgrade hook
#[post_upgrade]
fn post_upgrade(args: Option<InitArgs>) {
    service::init_upgrade::post_upgrade(args);
    ic_cdk::println!("ic_siwa_provider upgraded");
}

/// Prepare a SIWA login message for signing
///
/// # Arguments
/// * `address` - The Avalanche address (0x-prefixed, EIP-55 checksummed)
///
/// # Returns
/// * `Ok(PrepareLoginResponse)` - The message to sign, nonce, and expiration
/// * `Err(String)` - Error description if address is invalid
#[update]
async fn siwa_prepare_login(address: String) -> Result<PrepareLoginResponse, String> {
    service::siwa_prepare_login::prepare_login(address).await
}

/// Complete SIWA login with signed message
///
/// # Arguments
/// * `signature` - Hex-encoded signature from the user's wallet
/// * `address` - The Avalanche address that signed the message
/// * `session_key` - The session public key to bind to this authentication
///
/// # Returns
/// * `Ok(LoginResponse)` - The derived principal and session expiration
/// * `Err(String)` - Error if signature is invalid or session expired
#[update]
fn siwa_login(
    signature: String,
    address: String,
    session_key: Vec<u8>,
) -> Result<LoginResponse, String> {
    service::siwa_login::login(signature, address, session_key)
}

/// Get delegation for authenticated principal
///
/// # Arguments
/// * `address` - The Avalanche address
/// * `session_key` - The session public key from login
/// * `expiration` - Requested expiration timestamp (may be capped)
///
/// # Returns
/// * `Ok(SignedDelegation)` - The signed delegation for the session
/// * `Err(String)` - Error if not authenticated or expired
#[query]
fn siwa_get_delegation(
    address: String,
    session_key: Vec<u8>,
    expiration: u64,
) -> Result<SignedDelegation, String> {
    service::siwa_get_delegation::get_delegation(address, session_key, expiration)
}

/// Get ICP principal for Avalanche address
#[query]
fn get_principal(address: String) -> Result<Principal, String> {
    state::get_principal_for_address(&address)
        .ok_or_else(|| format!("No principal found for address {}", address))
}

/// Get Avalanche address for ICP principal
#[query]
fn get_address(principal: Principal) -> Result<String, String> {
    state::get_address_for_principal(&principal)
        .ok_or_else(|| format!("No address found for principal {}", principal))
}

/// Get Avalanche address for caller
#[query]
fn get_caller_address() -> Result<String, String> {
    let caller = ic_cdk::caller();
    state::get_address_for_principal(&caller)
        .ok_or_else(|| "No address found for caller".to_string())
}

/// Debug diagnostics response
#[derive(CandidType, serde::Serialize)]
pub struct DebugInfo {
    /// Domain configured for SIWA messages
    pub domain: String,
    /// URI configured for SIWA messages
    pub uri: String,
    /// Chain ID
    pub chain_id: u64,
    /// Session expiration time in nanoseconds
    pub session_expiration_ns: u64,
    /// Number of allowed domains configured
    pub allowed_domains_count: u64,
    /// Allowed domains (if debug enabled)
    pub allowed_domains: Vec<String>,
    /// Number of delegation targets configured
    pub delegation_targets_count: u64,
    /// Delegation targets (canister IDs)
    pub delegation_targets: Vec<Principal>,
    /// Rate limit: max logins per address
    pub rate_limit_per_address: u32,
    /// Rate limit: max total logins
    pub rate_limit_total: u32,
    /// Rate limit: window in seconds
    pub rate_limit_window_seconds: u64,
    /// Debug mode enabled
    pub debug_enabled: bool,
}

/// Get debug diagnostics (only available when debug=true in init args)
///
/// Returns configuration and state information for debugging.
/// This endpoint is disabled in production (when debug=false).
#[query]
fn debug_info() -> Result<DebugInfo, String> {
    let settings = state::get_settings();

    if !settings.debug {
        return Err("Debug endpoint disabled. Set debug=true in init args to enable.".to_string());
    }

    Ok(DebugInfo {
        domain: settings.domain.clone(),
        uri: settings.uri.clone(),
        chain_id: settings.chain_id,
        session_expiration_ns: settings.session_expiration_time,
        allowed_domains_count: settings.allowed_domains.len() as u64,
        allowed_domains: settings.allowed_domains.clone(),
        delegation_targets_count: settings.delegation_targets.len() as u64,
        delegation_targets: settings.delegation_targets.clone(),
        rate_limit_per_address: settings.rate_limits.max_logins_per_address,
        rate_limit_total: settings.rate_limits.max_logins_total,
        rate_limit_window_seconds: settings.rate_limits.window_seconds,
        debug_enabled: settings.debug,
    })
}

// Export Candid interface
ic_cdk::export_candid!();
