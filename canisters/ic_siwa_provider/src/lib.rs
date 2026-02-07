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
pub use service::siwa_prepare_login::{PrepareLoginRequest, PrepareLoginResponse};

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

/// Prepare a SIWA login message for signing (simple version)
///
/// Uses the canister's default domain/uri from init args.
/// For multi-tenant "SIWA as a Service", use `siwa_prepare_login_with_options` instead.
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

/// Prepare a SIWA login message with custom domain/uri (multi-tenant version)
///
/// This is the "SIWA as a Service" endpoint where each calling application
/// can specify its own domain/uri for the wallet signing prompt.
///
/// The domain must be in the `allowed_domains` whitelist configured at init.
///
/// # Arguments
/// * `request` - Login request containing:
///   - `address`: The Avalanche address (0x-prefixed)
///   - `domain`: Optional domain to show in wallet (must be whitelisted)
///   - `uri`: Optional URI to show in wallet
///
/// # Returns
/// * `Ok(PrepareLoginResponse)` - The message to sign, nonce, and expiration
/// * `Err(String)` - Error if address invalid or domain not whitelisted
#[update]
async fn siwa_prepare_login_with_options(
    request: PrepareLoginRequest,
) -> Result<PrepareLoginResponse, String> {
    service::siwa_prepare_login::prepare_login_with_options(request).await
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

/// Prepare a delegation for an authenticated session
///
/// This must be called before `siwa_get_delegation` to store the delegation
/// in the signature map for certified responses.
///
/// # Arguments
/// * `address` - The Avalanche address
/// * `session_key` - The session public key from login
/// * `expiration` - Requested expiration timestamp (may be capped)
///
/// # Returns
/// * `Ok(())` - Delegation prepared successfully
/// * `Err(String)` - Error if not authenticated or expired
#[update]
fn siwa_prepare_delegation(
    address: String,
    session_key: Vec<u8>,
    expiration: u64,
) -> Result<(), String> {
    service::siwa_prepare_delegation::prepare_delegation(address, session_key, expiration)
}

/// Get delegation for authenticated principal
///
/// This is a **query** call that retrieves the certified delegation.
/// You **must** call `siwa_prepare_delegation` first (an update call) and wait
/// for it to complete before calling this query.
///
/// # Arguments
/// * `address` - The Avalanche address
/// * `session_key` - The session public key from login
/// * `expiration` - Requested expiration timestamp (used for validation)
///
/// # Returns
/// * `Ok(SignedDelegation)` - The signed delegation for the session
/// * `Err(String)` - Error if not authenticated, expired, or delegation not prepared
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
    let caller = ic_cdk::api::msg_caller();
    state::get_address_for_principal(&caller)
        .ok_or_else(|| "No address found for caller".to_string())
}

/// Logout - revoke a specific session
///
/// The caller must provide the session key that was used during login.
/// This removes the auth session from the canister, effectively invalidating
/// any delegations created for that session.
///
/// # Arguments
/// * `address` - The Avalanche address
/// * `session_key` - The session public key from login
///
/// # Returns
/// * `Ok(())` - Session revoked
/// * `Err(String)` - Error if session not found
#[update]
fn siwa_logout(address: String, session_key: Vec<u8>) -> Result<(), String> {
    let key_hash = ic_siwa::siwa::hash_session_key(&session_key);

    // Verify the session belongs to this address
    let session =
        state::get_auth_session(&key_hash).ok_or_else(|| "Session not found".to_string())?;

    if session.address.to_lowercase() != address.to_lowercase() {
        return Err("Address does not match session".to_string());
    }

    state::remove_auth_session(&key_hash);
    Ok(())
}

/// Revoke all sessions for an address (controller-only)
///
/// Emergency endpoint to revoke all active sessions for a given address.
/// Only callable by canister controllers.
///
/// # Arguments
/// * `address` - The Avalanche address to revoke sessions for
///
/// # Returns
/// * `Ok(count)` - Number of sessions revoked
/// * `Err(String)` - Error if not a controller
#[update]
fn siwa_revoke_all(address: String) -> Result<u64, String> {
    let caller = ic_cdk::api::msg_caller();
    if !ic_cdk::api::is_controller(&caller) {
        return Err("Only canister controllers can revoke sessions".to_string());
    }

    let removed = state::remove_all_sessions_for_address(&address);
    Ok(removed as u64)
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
    /// Number of active login sessions
    pub login_sessions_count: u64,
    /// Number of active auth sessions
    pub auth_sessions_count: u64,
    /// Number of prepared delegations
    pub prepared_delegations_count: u64,
    /// Number of entries in signature map
    pub signature_map_count: u64,
}

/// Get debug diagnostics (restricted to canister controllers)
///
/// Returns configuration and state information for debugging.
/// Only callable by canister controllers for security.
#[query]
fn debug_info() -> Result<DebugInfo, String> {
    let caller = ic_cdk::api::msg_caller();
    if !ic_cdk::api::is_controller(&caller) {
        return Err("Only canister controllers can access debug info".to_string());
    }

    let settings = state::get_settings();

    let (
        login_sessions_count,
        auth_sessions_count,
        prepared_delegations_count,
        signature_map_count,
    ) = state::with_state(|s| {
        (
            s.login_sessions.len() as u64,
            s.auth_sessions.len() as u64,
            s.prepared_delegations.len() as u64,
            s.signature_map.len() as u64,
        )
    });

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
        login_sessions_count,
        auth_sessions_count,
        prepared_delegations_count,
        signature_map_count,
    })
}

// Export Candid interface
ic_cdk::export_candid!();
