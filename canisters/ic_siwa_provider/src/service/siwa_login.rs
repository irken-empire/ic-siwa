//! SIWA Login Service
//!
//! Verifies a signed SIWA message and creates an authenticated session.

use crate::state::{
    get_login_session, get_settings, remove_login_session, store_auth_session, AuthSession,
};
use candid::Principal;
use ic_siwa::siwa::{derive_principal, hash_session_key, validate_address};
use ic_siwa::SiwaMessage;

/// Login response with principal and expiration
#[derive(candid::CandidType, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct LoginResponse {
    /// The derived ICP principal for the user
    pub principal: Principal,
    /// Session expiration timestamp in nanoseconds
    pub expiration: u64,
}

/// Complete SIWA login with a signed message
///
/// # Arguments
/// * `signature` - Hex-encoded signature (0x-prefixed, 65 bytes)
/// * `address` - The Avalanche address that signed the message
/// * `session_key` - The session key to bind to this authentication
///
/// # Returns
/// * `Ok(LoginResponse)` - The derived principal and session expiration
/// * `Err(String)` - Error description
pub fn login(
    signature: String,
    address: String,
    session_key: Vec<u8>,
) -> Result<LoginResponse, String> {
    // Validate address format
    validate_address(&address).map_err(|e| e.to_string())?;

    // Get the pending login session for this address
    let login_session = get_login_session(&address)
        .ok_or_else(|| format!("No pending login session for address {}", address))?;

    // Check if session has expired
    let now = ic_cdk::api::time();
    if now > login_session.expires_at {
        remove_login_session(&address);
        return Err("Login session has expired".to_string());
    }

    // Get settings for principal derivation
    let settings = get_settings();

    // Reconstruct the SIWA message and verify the signature
    let siwa_message = SiwaMessage {
        domain: settings.domain.clone(),
        address: address.clone(),
        statement: Some("Sign in with Avalanche to the app.".to_string()),
        uri: settings.uri.clone(),
        version: "1".to_string(),
        chain_id: settings.chain_id,
        nonce: login_session.nonce.clone(),
        issued_at: extract_timestamp(&login_session.message, "Issued At: ")
            .ok_or("Failed to parse issued_at from stored message")?,
        expiration_time: extract_timestamp(&login_session.message, "Expiration Time: "),
        not_before: None,
        request_id: None,
        resources: None,
    };

    // Verify the signature and recover the address
    let recovered_address = siwa_message
        .verify_signature(&signature)
        .map_err(|e| format!("Signature verification failed: {}", e))?;

    // Ensure the recovered address matches the expected address (case-insensitive)
    if recovered_address.to_lowercase() != address.to_lowercase() {
        return Err(format!(
            "Recovered address {} does not match expected address {}",
            recovered_address, address
        ));
    }

    // Derive the ICP principal from the address and salt
    let principal = derive_principal(&address, &settings.salt)
        .map_err(|e| format!("Failed to derive principal: {}", e))?;

    // Calculate session expiration
    let session_expiration = now + settings.session_expiration_time;

    // Remove the used login session (prevent replay)
    remove_login_session(&address);

    // Store the authenticated session
    let key_hash = hash_session_key(&session_key);
    let auth_session = AuthSession {
        address: address.clone(),
        principal,
        session_key,
        created_at: now,
        expires_at: session_expiration,
    };
    store_auth_session(key_hash, auth_session);

    Ok(LoginResponse {
        principal,
        expiration: session_expiration,
    })
}

/// Extract a timestamp field from a SIWA message string
fn extract_timestamp(message: &str, prefix: &str) -> Option<String> {
    message
        .lines()
        .find(|line| line.starts_with(prefix))
        .map(|line| line.strip_prefix(prefix).unwrap_or(line).to_string())
}
