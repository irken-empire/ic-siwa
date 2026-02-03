//! Canister state management for IC-SIWA Provider
//!
//! Stores settings, active login sessions, and address-principal mappings.

use candid::{CandidType, Principal};
use ic_siwa::Settings;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;

/// Active login session (pending signature verification)
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct LoginSession {
    /// The Avalanche address attempting to log in
    pub address: String,
    /// The nonce for this session
    pub nonce: String,
    /// The full SIWA message to be signed
    pub message: String,
    /// Timestamp when this session was created (nanoseconds)
    pub created_at: u64,
    /// Timestamp when this session expires (nanoseconds)
    pub expires_at: u64,
}

/// Completed authentication session
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct AuthSession {
    /// The Avalanche address
    pub address: String,
    /// The derived ICP principal
    pub principal: Principal,
    /// Session key for delegation
    pub session_key: Vec<u8>,
    /// Timestamp when this session was created (nanoseconds)
    pub created_at: u64,
    /// Timestamp when this session expires (nanoseconds)
    pub expires_at: u64,
}

/// Global canister state
#[derive(Default)]
pub struct State {
    /// SIWA settings
    pub settings: Option<Settings>,
    /// Active login sessions keyed by address
    pub login_sessions: HashMap<String, LoginSession>,
    /// Authenticated sessions keyed by session key hash
    pub auth_sessions: HashMap<String, AuthSession>,
    /// Address to principal mapping
    pub address_to_principal: HashMap<String, Principal>,
    /// Principal to address mapping
    pub principal_to_address: HashMap<Principal, String>,
}

thread_local! {
    /// Thread-local state storage
    static STATE: RefCell<State> = RefCell::new(State::default());
}

impl State {
    /// Clean up expired login sessions
    pub fn cleanup_expired_logins(&mut self) {
        let now = ic_cdk::api::time();
        self.login_sessions
            .retain(|_, session| session.expires_at > now);
    }

    /// Clean up expired auth sessions
    pub fn cleanup_expired_auth(&mut self) {
        let now = ic_cdk::api::time();
        self.auth_sessions
            .retain(|_, session| session.expires_at > now);
    }
}

/// Read state immutably
pub fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&State) -> R,
{
    STATE.with(|state| f(&state.borrow()))
}

/// Mutate state
pub fn with_state_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut State) -> R,
{
    STATE.with(|state| f(&mut state.borrow_mut()))
}

/// Initialize state with settings
pub fn init_state(settings: Settings) {
    with_state_mut(|state| {
        state.settings = Some(settings);
        state.login_sessions.clear();
        state.auth_sessions.clear();
    });
}

/// Get current settings (panics if not initialized)
pub fn get_settings() -> Settings {
    with_state(|state| {
        state
            .settings
            .clone()
            .expect("Canister not initialized - settings not set")
    })
}

/// Get settings reference if initialized
pub fn try_get_settings() -> Option<Settings> {
    with_state(|state| state.settings.clone())
}

/// Store a new login session
pub fn store_login_session(session: LoginSession) {
    with_state_mut(|state| {
        // Clean up expired sessions first
        state.cleanup_expired_logins();
        // Store the new session
        state
            .login_sessions
            .insert(session.address.to_lowercase(), session);
    });
}

/// Get a login session by address
pub fn get_login_session(address: &str) -> Option<LoginSession> {
    with_state_mut(|state| {
        state.cleanup_expired_logins();
        state.login_sessions.get(&address.to_lowercase()).cloned()
    })
}

/// Remove a login session (after successful login)
pub fn remove_login_session(address: &str) {
    with_state_mut(|state| {
        state.login_sessions.remove(&address.to_lowercase());
    });
}

/// Store an authenticated session
pub fn store_auth_session(key_hash: String, session: AuthSession) {
    with_state_mut(|state| {
        state.cleanup_expired_auth();
        state.auth_sessions.insert(key_hash, session.clone());
        // Also store the address-principal mappings
        state
            .address_to_principal
            .insert(session.address.to_lowercase(), session.principal);
        state
            .principal_to_address
            .insert(session.principal, session.address);
    });
}

/// Get an auth session by session key hash
pub fn get_auth_session(key_hash: &str) -> Option<AuthSession> {
    with_state_mut(|state| {
        state.cleanup_expired_auth();
        state.auth_sessions.get(key_hash).cloned()
    })
}

/// Get principal for address
pub fn get_principal_for_address(address: &str) -> Option<Principal> {
    with_state(|state| {
        state
            .address_to_principal
            .get(&address.to_lowercase())
            .copied()
    })
}

/// Get address for principal
pub fn get_address_for_principal(principal: &Principal) -> Option<String> {
    with_state(|state| state.principal_to_address.get(principal).cloned())
}
