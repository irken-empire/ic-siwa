//! SIWA Prepare Login Service
//!
//! Generates a SIWA message for the user to sign with their Avalanche wallet.

use crate::state::{check_rate_limit, get_settings, store_login_session, LoginSession};
use ic_siwa::{siwa::validate_address, SiwaMessage};

/// Response from prepare_login
#[derive(candid::CandidType, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct PrepareLoginResponse {
    /// The SIWA message to be signed
    pub message: String,
    /// The nonce for this login attempt
    pub nonce: String,
    /// Expiration timestamp in nanoseconds
    pub expiration: u64,
}

/// Prepare a SIWA login message for the given address
///
/// # Arguments
/// * `address` - The Avalanche address (0x-prefixed)
///
/// # Returns
/// * `Ok(PrepareLoginResponse)` - The message to sign and nonce
/// * `Err(String)` - Error description
pub async fn prepare_login(address: String) -> Result<PrepareLoginResponse, String> {
    // Validate the address format
    validate_address(&address).map_err(|e| e.to_string())?;

    // Check rate limit before proceeding (this consumes cycles)
    check_rate_limit(&address)?;

    // Get settings
    let settings = get_settings();

    // Generate a unique nonce
    let nonce = ic_siwa::siwa::generate_nonce()
        .await
        .map_err(|e| format!("Failed to generate nonce: {}", e))?;

    // Create the SIWA message
    let siwa_message = SiwaMessage::new(&settings, &address, &nonce);
    let message_string = siwa_message.to_message();

    // Calculate expiration
    let now = ic_cdk::api::time();
    let expires_at = now + settings.login_expiration_time;

    // Store the login session
    let session = LoginSession {
        address: address.clone(),
        nonce: nonce.clone(),
        message: message_string.clone(),
        created_at: now,
        expires_at,
    };
    store_login_session(session);

    Ok(PrepareLoginResponse {
        message: message_string,
        nonce,
        expiration: expires_at,
    })
}
