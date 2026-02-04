//! SIWA Prepare Login Service
//!
//! Generates a SIWA message for the user to sign with their Avalanche wallet.
//! Supports multi-tenant "SIWA as a Service" where each calling application
//! can specify its own domain/uri for the wallet signing prompt.

use crate::state::{check_rate_limit, get_settings, store_login_session, LoginSession};
use ic_siwa::{siwa::validate_address, DomainValidator, SiwaMessage};

/// Request parameters for prepare_login
#[derive(candid::CandidType, serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct PrepareLoginRequest {
    /// The Avalanche address (0x-prefixed)
    pub address: String,
    /// Optional domain for the SIWA message (shown in wallet).
    /// Must be in allowed_domains whitelist. Falls back to canister default if not provided.
    pub domain: Option<String>,
    /// Optional URI for the SIWA message (shown in wallet).
    /// Falls back to canister default if not provided.
    pub uri: Option<String>,
}

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

/// Prepare a SIWA login message for the given address (simple version)
///
/// # Arguments
/// * `address` - The Avalanche address (0x-prefixed)
///
/// # Returns
/// * `Ok(PrepareLoginResponse)` - The message to sign and nonce
/// * `Err(String)` - Error description
pub async fn prepare_login(address: String) -> Result<PrepareLoginResponse, String> {
    prepare_login_with_options(PrepareLoginRequest {
        address,
        domain: None,
        uri: None,
    })
    .await
}

/// Prepare a SIWA login message with custom domain/uri options
///
/// This is the multi-tenant version for "SIWA as a Service" where each
/// calling application can specify its own domain/uri for the wallet prompt.
///
/// # Arguments
/// * `request` - The login request with address and optional domain/uri
///
/// # Returns
/// * `Ok(PrepareLoginResponse)` - The message to sign and nonce
/// * `Err(String)` - Error description
pub async fn prepare_login_with_options(
    request: PrepareLoginRequest,
) -> Result<PrepareLoginResponse, String> {
    // Validate the address format
    validate_address(&request.address).map_err(|e| e.to_string())?;

    // Check rate limit before proceeding (this consumes cycles)
    check_rate_limit(&request.address)?;

    // Get settings
    let settings = get_settings();

    // Determine the domain to use
    let domain = if let Some(ref requested_domain) = request.domain {
        // Validate the requested domain against allowed_domains whitelist
        if settings.allowed_domains.is_empty() {
            // No whitelist configured - allow any domain (not recommended for production)
            requested_domain.clone()
        } else {
            let validator = DomainValidator::new(&settings.allowed_domains);
            if !validator.is_allowed(requested_domain) {
                return Err(format!(
                    "Domain '{}' is not in the allowed domains list. \
                    Contact the SIWA provider to whitelist your domain.",
                    requested_domain
                ));
            }
            requested_domain.clone()
        }
    } else {
        // Fall back to canister default
        settings.domain.clone()
    };

    // Determine the URI to use
    let uri = request.uri.unwrap_or_else(|| settings.uri.clone());

    // Generate a unique nonce
    let nonce = ic_siwa::siwa::generate_nonce()
        .await
        .map_err(|e| format!("Failed to generate nonce: {}", e))?;

    // Create the SIWA message with the resolved domain/uri
    let siwa_message =
        SiwaMessage::new_with_domain(&settings, &request.address, &nonce, &domain, &uri);
    let message_string = siwa_message.to_message();

    // Calculate expiration
    let now = ic_cdk::api::time();
    let expires_at = now + settings.login_expiration_time;

    // Store the login session
    let session = LoginSession {
        address: request.address.clone(),
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
