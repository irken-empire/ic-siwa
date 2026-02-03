//! Test Canister (Rust)
//!
//! This canister is used for integration testing of the IC-SIWA authentication flow.
//! It provides endpoints to test the login flow and verify authentication.

use candid::Principal;
use ic_cdk_macros::{init, query, update};

/// Initialize the test canister
#[init]
fn init() {
    ic_cdk::println!("test_canister_rs initialized");
}

/// Health check endpoint
#[query]
fn health() -> String {
    "ok".to_string()
}

/// Test authentication - returns caller principal
#[query]
fn whoami() -> Principal {
    ic_cdk::caller()
}

/// Test protected endpoint - only accessible by authenticated users
#[query]
fn protected_data() -> Result<String, String> {
    let caller = ic_cdk::caller();
    if caller == Principal::anonymous() {
        Err("Unauthorized: anonymous caller".to_string())
    } else {
        Ok(format!("Protected data for principal: {}", caller))
    }
}

/// Test SIWA integration - prepare login
#[update]
async fn test_prepare_login(_provider_id: Principal, _address: String) -> Result<String, String> {
    // TODO: Call ic_siwa_provider.siwa_prepare_login
    Err("Not implemented".to_string())
}

/// Test SIWA integration - complete login
#[update]
async fn test_login(
    _provider_id: Principal,
    _signature: String,
    _address: String,
    _session_key: Vec<u8>,
) -> Result<Principal, String> {
    // TODO: Call ic_siwa_provider.siwa_login
    Err("Not implemented".to_string())
}

// Export Candid interface
ic_cdk::export_candid!();
