//! SIWA Prepare Login Service
//!
//! Generates a SIWA message for the user to sign with their Avalanche wallet.
//! Supports multi-tenant "SIWA as a Service" where each calling application
//! can specify its own domain/uri for the wallet signing prompt.

use crate::state::{check_rate_limit, store_login_session, with_settings, LoginSession};
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

    // Resolve domain, URI, and build SIWA message using settings (zero-copy access)
    let (domain, uri, login_expiration_time) = with_settings(|settings| {
        let domain = if let Some(ref requested_domain) = request.domain {
            // Validate the requested domain against allowed_domains whitelist
            if settings.allowed_domains.is_empty() {
                // No whitelist configured - reject custom domains to prevent phishing
                return Err("Custom domains require allowed_domains to be configured. \
                     Use siwa_prepare_login for the default domain, or configure \
                     allowed_domains in InitArgs for multi-tenant mode."
                    .to_string());
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

        let uri = request.uri.clone().unwrap_or_else(|| settings.uri.clone());
        Ok((domain, uri, settings.login_expiration_time))
    })?;

    // Generate a unique nonce (requires await, so must be outside with_settings)
    let nonce = ic_siwa::siwa::generate_nonce()
        .await
        .map_err(|e| format!("Failed to generate nonce: {}", e))?;

    // Create the SIWA message with the resolved domain/uri
    let message_string = with_settings(|settings| {
        let siwa_message =
            SiwaMessage::new_with_domain(settings, &request.address, &nonce, &domain, &uri);
        siwa_message.to_message()
    });

    // Calculate expiration
    let now = ic_cdk::api::time();
    let expires_at = now + login_expiration_time;

    // Store the login session, binding it to the caller who initiated it.
    // In multi-tenant mode (allowed_canisters configured), only this caller
    // can complete the login via siwa_login.
    let session = LoginSession {
        address: request.address.clone(),
        nonce: nonce.clone(),
        message: message_string.clone(),
        created_at: now,
        expires_at,
        initiator: ic_cdk::api::msg_caller(),
    };
    store_login_session(session)?;

    Ok(PrepareLoginResponse {
        message: message_string,
        nonce,
        expiration: expires_at,
    })
}
