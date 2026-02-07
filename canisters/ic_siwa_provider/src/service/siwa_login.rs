//! SIWA Login Service
//!
//! Verifies a signed SIWA message and creates an authenticated session.

use crate::state::{
    get_login_session, remove_login_session, store_auth_session, with_settings, AuthSession,
};
use candid::Principal;
use ic_siwa::siwa::{derive_principal, hash_session_key, validate_address};
use ic_siwa::types::DomainValidator;
use ic_siwa::{create_user_canister_pubkey, generate_seed, SiwaMessage};
use serde_bytes::ByteBuf;

/// Login response with principal, expiration, and canister public key
#[derive(candid::CandidType, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct LoginResponse {
    /// The derived ICP principal for the user
    pub user_principal: Principal,
    /// Session expiration timestamp in nanoseconds
    pub expiration: u64,
    /// The canister's public key for this user (used as root of delegation chain)
    pub user_canister_pubkey: ByteBuf,
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

    // In multi-tenant mode (allowed_canisters configured), verify the caller
    // is the same principal that initiated the login via prepare_login.
    // This prevents one whitelisted canister from completing a login flow
    // initiated by another. Controllers are always allowed.
    let caller = ic_cdk::api::msg_caller();
    let has_allowed_canisters = with_settings(|s| !s.allowed_canisters.is_empty());
    if has_allowed_canisters
        && caller != login_session.initiator
        && !ic_cdk::api::is_controller(&caller)
    {
        return Err("Login must be completed by the same caller that initiated it".to_string());
    }

    // Validate domain against settings whitelist (zero-copy access)
    with_settings(|settings| {
        if !settings.allowed_domains.is_empty() {
            let message_domain = extract_domain_from_message(&login_session.message)
                .ok_or("Failed to extract domain from SIWA message")?;

            let validator = DomainValidator::new(&settings.allowed_domains);
            if !validator.is_allowed(&message_domain) {
                crate::state::debug_log!(
                    "[SECURITY] Domain rejected: '{}' not in allowed list for address {}",
                    message_domain,
                    address
                );
                return Err(format!(
                    "Domain '{}' is not allowed. This canister only accepts logins from configured domains.",
                    message_domain
                ));
            }
        }
        Ok(())
    })?;

    // Parse the stored SIWA message directly instead of reconstructing from settings.
    // This ensures we verify the signature against the exact message the user signed,
    // even if canister settings (domain, URI, etc.) changed between prepare_login and login,
    // or if a multi-tenant domain/URI was used during prepare_login.
    let siwa_message = SiwaMessage::from_message(&login_session.message)
        .map_err(|e| format!("Failed to parse stored SIWA message: {}", e))?;

    // Validate chain ID matches settings (defense-in-depth against settings changes between prepare and login)
    let chain_id = with_settings(|s| s.chain_id);
    if siwa_message.chain_id != chain_id {
        return Err(format!(
            "Chain ID mismatch: message has {} but settings require {}",
            siwa_message.chain_id, chain_id
        ));
    }

    // Validate SIWA version
    if siwa_message.version != "1" {
        return Err(format!(
            "Unsupported SIWA version: {}",
            siwa_message.version
        ));
    }

    // Validate message expiration time (defense-in-depth, per EIP-4361).
    // The login session expiration (checked above) is derived from the same
    // login_expiration_time setting, so in practice they are equivalent.
    // This explicit check guards against any future divergence.
    if let Some(ref exp_time) = siwa_message.expiration_time {
        if let Some(exp_secs) = ic_siwa::siwa::parse_timestamp(exp_time) {
            let now_secs = now / 1_000_000_000;
            if now_secs > exp_secs {
                remove_login_session(&address);
                return Err("SIWA message has expired".to_string());
            }
        }
    }

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

    // Derive the ICP principal and canister pubkey using settings (zero-copy access)
    let (principal, user_canister_pubkey, session_expiration_time) =
        with_settings(|settings| -> Result<_, String> {
            let principal = derive_principal(&address, &settings.salt)
                .map_err(|e| format!("Failed to derive principal: {}", e))?;

            let seed = generate_seed(&settings.salt, &address);
            let canister_id = ic_cdk::api::canister_self();
            let user_canister_pubkey = create_user_canister_pubkey(&canister_id, &seed)
                .map_err(|e| format!("Failed to create user canister pubkey: {}", e))?;

            Ok((
                principal,
                user_canister_pubkey,
                settings.session_expiration_time,
            ))
        })?;

    // Calculate session expiration
    let session_expiration = now + session_expiration_time;

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
    store_auth_session(key_hash, auth_session)?;

    // Note: The delegation is stored in the signature map when get_delegation is called,
    // not here. This allows flexible expiration times in the delegation.

    Ok(LoginResponse {
        user_principal: principal,
        expiration: session_expiration,
        user_canister_pubkey: ByteBuf::from(user_canister_pubkey),
    })
}

/// Extract the domain from a SIWA message
/// The first line format is: "domain wants you to sign in with your Avalanche account:"
fn extract_domain_from_message(message: &str) -> Option<String> {
    let first_line = message.lines().next()?;
    // Pattern: "domain wants you to sign in with your Avalanche account:"
    let suffix = " wants you to sign in with your Avalanche account:";
    if first_line.ends_with(suffix) {
        let domain = first_line.strip_suffix(suffix)?;
        Some(domain.to_lowercase())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain_from_message() {
        let message = "example.com wants you to sign in with your Avalanche account:\n0x1234...";
        assert_eq!(
            extract_domain_from_message(message),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn test_extract_domain_from_message_with_subdomain() {
        let message =
            "app.example.com wants you to sign in with your Avalanche account:\n0x1234...";
        assert_eq!(
            extract_domain_from_message(message),
            Some("app.example.com".to_string())
        );
    }

    #[test]
    fn test_extract_domain_from_message_invalid() {
        let message = "invalid message format";
        assert_eq!(extract_domain_from_message(message), None);
    }
}
