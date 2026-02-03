//! Test Canister (Rust)
//!
//! This canister is used for integration testing of the IC-SIWA authentication flow.
//! It provides endpoints to test the login flow and verify authentication.

use candid::{CandidType, Deserialize, Principal};
use ic_cdk_macros::{init, query, update};

/// Response types matching ic_siwa_provider
#[derive(CandidType, Deserialize)]
pub enum PrepareLoginResponse {
    Ok(String),
    Err(String),
}

#[derive(CandidType, Deserialize)]
pub enum LoginResponse {
    Ok(Principal),
    Err(String),
}

#[derive(CandidType, Deserialize)]
pub struct SignedDelegation {
    pub delegation: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(CandidType, Deserialize)]
pub enum GetDelegationResponse {
    Ok(SignedDelegation),
    Err(String),
}

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

/// Get caller principal - useful for testing authentication
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
/// Calls ic_siwa_provider.siwa_prepare_login
#[update]
async fn test_prepare_login(provider_id: Principal, address: String) -> Result<String, String> {
    let result: Result<(PrepareLoginResponse,), _> =
        ic_cdk::call(provider_id, "siwa_prepare_login", (address,)).await;

    match result {
        Ok((PrepareLoginResponse::Ok(message),)) => Ok(message),
        Ok((PrepareLoginResponse::Err(e),)) => Err(format!("Provider error: {}", e)),
        Err((code, msg)) => Err(format!("Call failed: {:?} - {}", code, msg)),
    }
}

/// Test SIWA integration - complete login
/// Calls ic_siwa_provider.siwa_login
#[update]
async fn test_login(
    provider_id: Principal,
    signature: String,
    address: String,
    session_key: Vec<u8>,
) -> Result<Principal, String> {
    let result: Result<(LoginResponse,), _> =
        ic_cdk::call(provider_id, "siwa_login", (signature, address, session_key)).await;

    match result {
        Ok((LoginResponse::Ok(principal),)) => Ok(principal),
        Ok((LoginResponse::Err(e),)) => Err(format!("Provider error: {}", e)),
        Err((code, msg)) => Err(format!("Call failed: {:?} - {}", code, msg)),
    }
}

/// Test SIWA integration - get delegation
/// Calls ic_siwa_provider.siwa_get_delegation
#[update]
async fn test_get_delegation(
    provider_id: Principal,
    address: String,
    session_key: Vec<u8>,
    expiration: u64,
) -> Result<SignedDelegation, String> {
    let result: Result<(GetDelegationResponse,), _> = ic_cdk::call(
        provider_id,
        "siwa_get_delegation",
        (address, session_key, expiration),
    )
    .await;

    match result {
        Ok((GetDelegationResponse::Ok(delegation),)) => Ok(delegation),
        Ok((GetDelegationResponse::Err(e),)) => Err(format!("Provider error: {}", e)),
        Err((code, msg)) => Err(format!("Call failed: {:?} - {}", code, msg)),
    }
}

/// Serve simple HTML test page
#[query]
fn get_test_page() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>IC-SIWA Rust Test</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 600px;
            margin: 50px auto;
            padding: 20px;
            background: #1a1a2e;
            color: #eee;
        }
        .card {
            background: #16213e;
            border-radius: 12px;
            padding: 24px;
            margin-bottom: 20px;
        }
        h1 { color: #e94560; margin-bottom: 8px; }
        .btn {
            background: #e94560;
            color: white;
            border: none;
            padding: 12px 24px;
            border-radius: 8px;
            cursor: pointer;
            font-size: 16px;
            width: 100%;
        }
        .btn:hover { background: #ff6b6b; }
        .btn:disabled { background: #555; cursor: not-allowed; }
        .status {
            padding: 12px;
            border-radius: 8px;
            margin-top: 16px;
        }
        .success { background: #0f5132; color: #75b798; }
        .error { background: #842029; color: #f5c2c7; }
        .info { background: #055160; color: #6edff6; }
        pre {
            background: #0f0f23;
            padding: 12px;
            border-radius: 8px;
            overflow-x: auto;
            font-size: 12px;
        }
        .hidden { display: none; }
    </style>
</head>
<body>
    <div class="card">
        <h1>IC-SIWA Test</h1>
        <p>Rust canister integration test</p>
    </div>

    <div class="card">
        <h2>1. Check Health</h2>
        <button class="btn" onclick="checkHealth()">Check Health</button>
        <div id="health-result" class="status hidden"></div>
    </div>

    <div class="card">
        <h2>2. Who Am I?</h2>
        <button class="btn" onclick="checkWhoami()">Get Principal</button>
        <div id="whoami-result" class="status hidden"></div>
    </div>

    <div class="card">
        <h2>3. Protected Data</h2>
        <button class="btn" onclick="getProtected()">Get Protected Data</button>
        <div id="protected-result" class="status hidden"></div>
    </div>

    <script type="module">
        // Simple test functions
        window.checkHealth = async () => {
            const result = document.getElementById('health-result');
            result.className = 'status info';
            result.textContent = 'Health: ok';
            result.classList.remove('hidden');
        };

        window.checkWhoami = async () => {
            const result = document.getElementById('whoami-result');
            result.className = 'status info';
            result.textContent = 'Use dfx canister call test_canister_rs whoami to test';
            result.classList.remove('hidden');
        };

        window.getProtected = async () => {
            const result = document.getElementById('protected-result');
            result.className = 'status info';
            result.textContent = 'Use dfx canister call test_canister_rs protected_data to test';
            result.classList.remove('hidden');
        };
    </script>
</body>
</html>"#
        .to_string()
}

// Export Candid interface
ic_cdk::export_candid!();
