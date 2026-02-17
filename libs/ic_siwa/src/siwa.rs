//! SIWA (Sign-In with Avalanche) message handling
//!
//! Implements EIP-4361 (Sign-In with Ethereum) adapted for Avalanche C-Chain.
//! Uses the Ethereum message signing prefix for maximum wallet compatibility.

use crate::error::SiwaError;
use crate::hash::{hash_personal_message, keccak256};
use crate::settings::Settings;
use crate::types::Nonce;
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};

/// SIWA message for signing
#[derive(Clone, Debug)]
pub struct SiwaMessage {
    /// Domain the message is for
    pub domain: String,
    /// Avalanche address (0x-prefixed, EIP-55 checksummed)
    pub address: String,
    /// Statement/description
    pub statement: Option<String>,
    /// URI
    pub uri: String,
    /// Version (always "1")
    pub version: String,
    /// Chain ID (43114 for Avalanche Mainnet, 43113 for Fuji)
    pub chain_id: u64,
    /// Unique nonce
    pub nonce: Nonce,
    /// Issued at timestamp (ISO 8601)
    pub issued_at: String,
    /// Expiration timestamp (ISO 8601)
    pub expiration_time: Option<String>,
    /// Not before timestamp (ISO 8601)
    pub not_before: Option<String>,
    /// Request ID
    pub request_id: Option<String>,
    /// Resources
    pub resources: Option<Vec<String>>,
}

impl SiwaMessage {
    /// Create a new SIWA message using settings' default domain/uri
    pub fn new(settings: &Settings, address: &str, nonce: &str) -> Self {
        Self::new_with_domain(settings, address, nonce, &settings.domain, &settings.uri)
    }

    /// Create a new SIWA message with a custom domain and URI
    ///
    /// This is used for multi-tenant "SIWA as a Service" scenarios where
    /// each calling application can specify its own domain/uri for the
    /// wallet signing prompt.
    ///
    /// # Arguments
    /// * `settings` - Canister settings (for chain_id, expiration times, etc.)
    /// * `address` - The Avalanche address (0x-prefixed)
    /// * `nonce` - Unique nonce for this login attempt
    /// * `domain` - Domain to show in wallet (e.g., "game.tresr.community")
    /// * `uri` - URI to show in wallet (e.g., "https://game.tresr.community")
    pub fn new_with_domain(
        settings: &Settings,
        address: &str,
        nonce: &str,
        domain: &str,
        uri: &str,
    ) -> Self {
        // Get current time in nanoseconds from IC
        let now_ns = ic_cdk::api::time();
        let now_secs = now_ns / 1_000_000_000;

        // Calculate expiration
        let exp_ns = now_ns + settings.login_expiration_time;
        let exp_secs = exp_ns / 1_000_000_000;

        Self {
            domain: domain.to_string(),
            address: address.to_string(),
            statement: Some("Sign in with Avalanche to the app.".to_string()),
            uri: uri.to_string(),
            version: "1".to_string(),
            chain_id: settings.chain_id,
            nonce: nonce.to_string(),
            issued_at: format_timestamp(now_secs),
            expiration_time: Some(format_timestamp(exp_secs)),
            not_before: None,
            request_id: None,
            resources: None,
        }
    }

    /// Convert message to EIP-4361 format string for signing
    pub fn to_message(&self) -> String {
        let mut msg = format!(
            "{} wants you to sign in with your Avalanche account:\n{}\n",
            self.domain, self.address
        );

        if let Some(ref statement) = self.statement {
            msg.push('\n');
            msg.push_str(statement);
            msg.push('\n');
        }

        msg.push_str(&format!("\nURI: {}", self.uri));
        msg.push_str(&format!("\nVersion: {}", self.version));
        msg.push_str(&format!("\nChain ID: {}", self.chain_id));
        msg.push_str(&format!("\nNonce: {}", self.nonce));
        msg.push_str(&format!("\nIssued At: {}", self.issued_at));

        if let Some(ref exp) = self.expiration_time {
            msg.push_str(&format!("\nExpiration Time: {}", exp));
        }

        if let Some(ref nb) = self.not_before {
            msg.push_str(&format!("\nNot Before: {}", nb));
        }

        if let Some(ref rid) = self.request_id {
            msg.push_str(&format!("\nRequest ID: {}", rid));
        }

        if let Some(ref resources) = self.resources {
            msg.push_str("\nResources:");
            for resource in resources {
                msg.push_str(&format!("\n- {}", resource));
            }
        }

        msg
    }

    /// Parse a SIWA message from its EIP-4361 format string representation
    ///
    /// This is the inverse of `to_message()`. It parses the stored message string
    /// back into a `SiwaMessage` struct, preserving the exact domain, URI, and
    /// other fields that were present when the message was originally created.
    ///
    /// # Arguments
    /// * `text` - The EIP-4361 format message string
    ///
    /// # Returns
    /// * `Ok(SiwaMessage)` - The parsed message
    /// * `Err(SiwaError)` - If the message format is invalid
    pub fn from_message(text: &str) -> Result<Self, SiwaError> {
        let lines: Vec<&str> = text.lines().collect();

        // First line: "{domain} wants you to sign in with your Avalanche account:"
        let first_line = lines
            .first()
            .ok_or_else(|| SiwaError::InvalidMessage("Empty message".to_string()))?;
        let suffix = " wants you to sign in with your Avalanche account:";
        let domain = first_line
            .strip_suffix(suffix)
            .ok_or_else(|| SiwaError::InvalidMessage("Invalid first line format".to_string()))?
            .to_string();

        // Second line: address
        let address = lines
            .get(1)
            .ok_or_else(|| SiwaError::InvalidMessage("Missing address line".to_string()))?
            .to_string();

        // Parse statement: lines between address and the first field line (starting with "URI:")
        // The format is: empty line, statement text, empty line, then fields
        let mut statement = None;
        let mut field_start = 2;
        for (i, line) in lines.iter().enumerate().skip(2) {
            if line.starts_with("URI: ") {
                // Check if there's a non-empty statement between address and URI
                // Skip leading/trailing empty lines in the statement region
                let statement_lines: Vec<&str> = lines[2..i]
                    .iter()
                    .copied()
                    .filter(|l| !l.is_empty())
                    .collect();
                if !statement_lines.is_empty() {
                    statement = Some(statement_lines.join("\n"));
                }
                field_start = i;
                break;
            }
        }

        // Parse key-value fields
        let mut uri = None;
        let mut version = "1".to_string();
        let mut chain_id = 0u64;
        let mut nonce = String::new();
        let mut issued_at = String::new();
        let mut expiration_time = None;
        let mut not_before = None;
        let mut request_id = None;
        let mut resources = None;

        let mut i = field_start;
        while i < lines.len() {
            let line = lines[i];
            if let Some(val) = line.strip_prefix("URI: ") {
                uri = Some(val.to_string());
            } else if let Some(val) = line.strip_prefix("Version: ") {
                version = val.to_string();
            } else if let Some(val) = line.strip_prefix("Chain ID: ") {
                chain_id = val
                    .parse::<u64>()
                    .map_err(|_| SiwaError::InvalidMessage("Invalid chain ID".to_string()))?;
            } else if let Some(val) = line.strip_prefix("Nonce: ") {
                nonce = val.to_string();
            } else if let Some(val) = line.strip_prefix("Issued At: ") {
                issued_at = val.to_string();
            } else if let Some(val) = line.strip_prefix("Expiration Time: ") {
                expiration_time = Some(val.to_string());
            } else if let Some(val) = line.strip_prefix("Not Before: ") {
                not_before = Some(val.to_string());
            } else if let Some(val) = line.strip_prefix("Request ID: ") {
                request_id = Some(val.to_string());
            } else if line == "Resources:" {
                let mut res = Vec::new();
                i += 1;
                while i < lines.len() {
                    if let Some(val) = lines[i].strip_prefix("- ") {
                        res.push(val.to_string());
                    } else {
                        break;
                    }
                    i += 1;
                }
                resources = Some(res);
                continue;
            }
            i += 1;
        }

        let uri = uri.ok_or_else(|| SiwaError::InvalidMessage("Missing URI field".to_string()))?;

        if nonce.is_empty() {
            return Err(SiwaError::InvalidMessage("Missing Nonce field".to_string()));
        }
        if issued_at.is_empty() {
            return Err(SiwaError::InvalidMessage(
                "Missing Issued At field".to_string(),
            ));
        }

        Ok(Self {
            domain,
            address,
            statement,
            uri,
            version,
            chain_id,
            nonce,
            issued_at,
            expiration_time,
            not_before,
            request_id,
            resources,
        })
    }

    /// Verify message signature and recover the signer's address
    ///
    /// # Arguments
    /// * `signature` - Hex-encoded signature (0x-prefixed, 130 or 132 chars)
    ///
    /// # Returns
    /// The recovered address if signature is valid and matches self.address
    pub fn verify_signature(&self, signature: &str) -> Result<String, SiwaError> {
        // Parse signature
        let sig_bytes = parse_signature(signature)?;

        // Hash the message using Ethereum personal sign prefix
        let message = self.to_message();
        let message_hash = hash_personal_message(&message);

        // Recover the public key from the signature
        let recovered_address = recover_address(&message_hash, &sig_bytes)?;

        // Verify the recovered address matches the expected address (case-insensitive)
        if recovered_address.to_lowercase() != self.address.to_lowercase() {
            return Err(SiwaError::InvalidSignature(format!(
                "Address mismatch: expected {}, got {}",
                self.address, recovered_address
            )));
        }

        Ok(recovered_address)
    }
}

/// Parse a hex-encoded signature into bytes
fn parse_signature(signature: &str) -> Result<[u8; 65], SiwaError> {
    // Remove 0x prefix if present
    let sig_hex = signature.strip_prefix("0x").unwrap_or(signature);

    // Signature should be 130 hex chars (65 bytes)
    if sig_hex.len() != 130 {
        return Err(SiwaError::InvalidSignature(format!(
            "Invalid signature length: expected 130 hex chars, got {}",
            sig_hex.len()
        )));
    }

    let sig_bytes = hex::decode(sig_hex)
        .map_err(|e| SiwaError::InvalidSignature(format!("Invalid hex: {}", e)))?;

    let mut result = [0u8; 65];
    result.copy_from_slice(&sig_bytes);
    Ok(result)
}

/// Recover address from message hash and signature
fn recover_address(message_hash: &[u8; 32], signature: &[u8; 65]) -> Result<String, SiwaError> {
    // Extract r, s, v from signature
    // signature[0..32] = r
    // signature[32..64] = s
    // signature[64] = v (recovery id)
    let r_s = &signature[0..64];
    let v = signature[64];

    // Ethereum signatures use v = 27 or 28, normalize to 0 or 1
    let recovery_id = if v >= 27 { v - 27 } else { v };

    let recovery_id = RecoveryId::try_from(recovery_id)
        .map_err(|_| SiwaError::InvalidSignature("Invalid recovery ID".to_string()))?;

    let signature = Signature::from_slice(r_s)
        .map_err(|e| SiwaError::InvalidSignature(format!("Invalid signature bytes: {}", e)))?;

    // Recover the public key
    let verifying_key = VerifyingKey::recover_from_prehash(message_hash, &signature, recovery_id)
        .map_err(|e| {
        SiwaError::InvalidSignature(format!("Failed to recover public key: {}", e))
    })?;

    // Derive address from public key
    let address = derive_address_from_pubkey(&verifying_key)?;

    Ok(address)
}

/// Derive Ethereum/Avalanche address from public key
fn derive_address_from_pubkey(pubkey: &VerifyingKey) -> Result<String, SiwaError> {
    // Get uncompressed public key bytes (65 bytes: 0x04 || x || y)
    let pubkey_bytes = pubkey.to_encoded_point(false);
    let pubkey_bytes = pubkey_bytes.as_bytes();

    // Skip the 0x04 prefix and hash the 64 bytes (x || y)
    let hash = keccak256(&pubkey_bytes[1..]);

    // Take last 20 bytes as address
    let address_bytes = &hash[12..32];

    // Convert to EIP-55 checksummed address
    let address = to_eip55_checksum(address_bytes);

    Ok(address)
}

/// Convert address bytes to EIP-55 checksummed hex string
fn to_eip55_checksum(address_bytes: &[u8]) -> String {
    let address_hex = hex::encode(address_bytes);
    let address_hash = keccak256(address_hex.as_bytes());

    let mut checksummed = String::with_capacity(42);
    checksummed.push_str("0x");

    for (i, c) in address_hex.chars().enumerate() {
        let hash_nibble = if i.is_multiple_of(2) {
            (address_hash[i / 2] >> 4) & 0x0f
        } else {
            address_hash[i / 2] & 0x0f
        };

        if c.is_ascii_alphabetic() && hash_nibble >= 8 {
            checksummed.push(c.to_ascii_uppercase());
        } else {
            checksummed.push(c);
        }
    }

    checksummed
}

/// Validate an Avalanche/Ethereum address (0x-prefixed, 42 chars, valid hex)
pub fn validate_address(address: &str) -> Result<(), SiwaError> {
    if !address.starts_with("0x") {
        return Err(SiwaError::InvalidAddress(
            "Address must start with 0x".to_string(),
        ));
    }

    if address.len() != 42 {
        return Err(SiwaError::InvalidAddress(format!(
            "Address must be 42 characters, got {}",
            address.len()
        )));
    }

    // Validate hex
    hex::decode(&address[2..])
        .map_err(|e| SiwaError::InvalidAddress(format!("Invalid hex: {}", e)))?;

    // Optionally verify EIP-55 checksum
    let address_bytes = hex::decode(&address[2..]).unwrap();
    let checksummed = to_eip55_checksum(&address_bytes);

    // Only enforce checksum if address has mixed case
    let has_upper = address[2..].chars().any(|c| c.is_ascii_uppercase());
    let has_lower = address[2..].chars().any(|c| c.is_ascii_lowercase());

    if has_upper && has_lower && address != checksummed {
        return Err(SiwaError::InvalidAddress(
            "Invalid EIP-55 checksum".to_string(),
        ));
    }

    Ok(())
}

/// Format Unix timestamp as ISO 8601 string (UTC)
pub fn format_timestamp(timestamp_secs: u64) -> String {
    // Calculate date/time components from Unix timestamp
    // This is a simplified implementation for WASM environment
    let days_since_epoch = timestamp_secs / 86400;
    let time_of_day = timestamp_secs % 86400;

    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calculate year, month, day using a simplified algorithm
    let (year, month, day) = days_to_ymd(days_since_epoch);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch to year, month, day
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Simplified calculation - good enough for our purposes
    let mut remaining_days = days as i64;
    let mut year = 1970i64;

    // Find the year
    loop {
        let days_in_year = if is_leap_year(year as u64) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    // Find the month
    let days_in_months: [i64; 12] = if is_leap_year(year as u64) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u64;
    for days_in_month in days_in_months.iter() {
        if remaining_days < *days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }

    let day = remaining_days as u64 + 1;

    (year as u64, month, day)
}

/// Check if a year is a leap year
fn is_leap_year(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Parse an ISO 8601 UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`) into Unix seconds.
///
/// This is the inverse of [`format_timestamp`] and only supports the exact
/// format produced by this crate. Returns `None` on malformed input.
pub fn parse_timestamp(s: &str) -> Option<u64> {
    // Expected: "YYYY-MM-DDTHH:MM:SSZ" (20 chars)
    if s.len() != 20 || !s.ends_with('Z') {
        return None;
    }
    let b = s.as_bytes();
    if b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }

    let year: u64 = s[0..4].parse().ok()?;
    let month: u64 = s[5..7].parse().ok()?;
    let day: u64 = s[8..10].parse().ok()?;
    let hours: u64 = s[11..13].parse().ok()?;
    let minutes: u64 = s[14..16].parse().ok()?;
    let seconds: u64 = s[17..19].parse().ok()?;

    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..=23).contains(&hours)
        || !(0..=59).contains(&minutes)
        || !(0..=59).contains(&seconds)
    {
        return None;
    }

    // Days from epoch to start of year
    let mut days = 0u64;
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    // Days from start of year to start of month
    let days_in_months: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    for &d in &days_in_months[..((month - 1) as usize)] {
        days += d;
    }
    days += day - 1;

    Some(days * 86400 + hours * 3600 + minutes * 60 + seconds)
}

/// Generate a random nonce using IC randomness
pub async fn generate_nonce() -> Result<Nonce, SiwaError> {
    // Use IC management canister for randomness
    let random_bytes: Vec<u8> = ic_cdk::management_canister::raw_rand()
        .await
        .map_err(|e| SiwaError::InternalError(format!("Failed to generate randomness: {:?}", e)))?;

    // Take first 16 bytes and encode as hex
    let nonce_bytes = &random_bytes[..16.min(random_bytes.len())];
    Ok(hex::encode(nonce_bytes))
}

/// Derive an ICP principal from an Avalanche address and salt
///
/// The derivation is deterministic: same address + salt = same principal.
/// The resulting principal is opaque (not self-authenticating).
pub fn derive_principal(address: &str, salt: &str) -> Result<candid::Principal, SiwaError> {
    // Normalize the address to lowercase
    let normalized_address = address.to_lowercase();

    // Create the seed by hashing address + salt
    let seed_input = format!("{}{}", normalized_address, salt);
    let seed = keccak256(seed_input.as_bytes());

    // Create an opaque principal from the seed
    // IC principals can be at most 29 bytes
    // We use the first 28 bytes of the hash for the principal data
    let principal_bytes = &seed[..28];

    candid::Principal::try_from_slice(principal_bytes)
        .map_err(|e| SiwaError::InternalError(format!("Failed to create principal: {}", e)))
}

/// Get the session key hash for storage lookup
pub fn hash_session_key(session_key: &[u8]) -> String {
    hex::encode(&keccak256(session_key)[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    // Well-known EIP-55 test vector addresses (not secrets)
    const TEST_ADDRESS_BYTES_HEX: &str = "5aaeb6053f3e94c9b9a09f33669435e7ef1beaed";
    const ZERO_ADDRESS_BYTES_HEX: &str = "0000000000000000000000000000000000000000";

    // Tests for private helper functions that cannot be moved to
    // integration tests. These do NOT use hardcoded salts so they
    // will not trigger CodeQL false positives.
    //
    // All public-function tests (derive_principal, validate_address,
    // SiwaMessage, hash_session_key, format_timestamp, parse_timestamp)
    // have been moved to tests/test_siwa.rs.

    #[test]
    fn test_to_eip55_checksum() {
        let bytes = hex::decode(TEST_ADDRESS_BYTES_HEX).unwrap();
        let checksummed = to_eip55_checksum(&bytes);
        assert_eq!(checksummed, "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed");
    }

    #[test]
    fn test_eip55_checksum_all_lowercase() {
        let bytes = hex::decode(ZERO_ADDRESS_BYTES_HEX).unwrap();
        let checksummed = to_eip55_checksum(&bytes);
        assert!(checksummed.starts_with("0x"));
        assert_eq!(checksummed.len(), 42);
    }

    #[test]
    fn test_parse_signature_valid() {
        let sig = "0x".to_string() + &"a".repeat(130);
        assert!(parse_signature(&sig).is_ok());
    }

    #[test]
    fn test_parse_signature_invalid_length() {
        let sig = "0x".to_string() + &"a".repeat(128);
        assert!(parse_signature(&sig).is_err());
    }

    #[test]
    fn test_parse_signature_without_prefix() {
        let sig = "a".repeat(130);
        assert!(parse_signature(&sig).is_ok());
    }

    #[test]
    fn test_parse_signature_wrong_hex() {
        let sig = "0x".to_string() + &"g".repeat(130);
        assert!(parse_signature(&sig).is_err());
    }

    #[test]
    fn test_days_to_ymd_epoch() {
        let (year, month, day) = days_to_ymd(0);
        assert_eq!((year, month, day), (1970, 1, 1));
    }

    #[test]
    fn test_days_to_ymd_leap_year() {
        assert!(is_leap_year(2000));
        let (year, month, day) = days_to_ymd(10957);
        assert_eq!(year, 2000);
        assert_eq!(month, 1);
        assert_eq!(day, 1);
    }

    #[test]
    fn test_is_leap_year() {
        assert!(!is_leap_year(1970));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
    }

    // EIP-55 checksum validation tests
    #[test]
    fn test_validate_address_all_lowercase() {
        // All lowercase is valid per EIP-55 (no checksum required)
        let addr = "0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed";
        assert!(validate_address(addr).is_ok());
    }

    #[test]
    fn test_validate_address_all_uppercase() {
        // All uppercase is valid per EIP-55 (no checksum required)
        let addr = "0x5AAEB6053F3E94C9B9A09F33669435E7EF1BEAED";
        assert!(validate_address(addr).is_ok());
    }

    #[test]
    fn test_validate_address_valid_mixed_case() {
        // Valid EIP-55 checksummed mixed-case address
        let addr = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed";
        assert!(validate_address(addr).is_ok());
    }

    #[test]
    fn test_validate_address_invalid_mixed_case() {
        // Invalid mixed-case (deliberately wrong checksum)
        let addr = "0x5aAEB6053F3E94C9b9A09f33669435E7Ef1BeAed";
        assert!(validate_address(addr).is_err());
    }

    #[test]
    fn test_validate_address_digits_only() {
        // Address with only hex digits (no letters) — valid without checksum
        let addr = "0x0000000000000000000000000000000000000001";
        assert!(validate_address(addr).is_ok());
    }
}
