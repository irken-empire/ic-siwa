//! Custom getrandom implementation for Internet Computer
//!
//! Provides a thread-local CSPRNG for synchronous randomness on the IC.
//!
//! # Security Model
//!
//! The IC does not provide a synchronous random source. Libraries like `k256`
//! (via `rand_core` / `getrandom`) may require synchronous randomness for
//! operations such as side-channel blinding during ECDSA.
//!
//! This module implements a SHA-256 based CSPRNG in counter mode:
//!
//! 1. **Initial seed**: IC time + caller principal hash (weak, but functional
//!    before `raw_rand` is available).
//! 2. **Strong reseed**: Call [`seed_rng`] during `init` / `post_upgrade` to
//!    mix in 32 bytes from the IC management canister's `raw_rand()`.
//! 3. **Forward secrecy**: Each output generation mixes in the current IC
//!    timestamp and increments a monotonic counter, then re-keys the internal
//!    state from the hash output.
//!
//! # Audit Note (Ticket #015)
//!
//! `k256` 0.13's `VerifyingKey::recover_from_prehash()` — the only k256
//! operation used in this canister — is **deterministic** and does **not**
//! call `getrandom`. Therefore the quality of this CSPRNG does not affect
//! ECDSA signature verification security. This implementation exists as a
//! defense-in-depth measure for any future dependency that may call
//! `getrandom` at runtime.

use getrandom::{register_custom_getrandom, Error};
use sha2::{Digest, Sha256};
use std::cell::RefCell;

/// Internal CSPRNG state.
///
/// Uses SHA-256 in counter mode with re-keying after every generation.
struct CsprngState {
    /// 256-bit key material (re-keyed after every output block)
    key: [u8; 32],
    /// Monotonic counter (never resets, prevents identical outputs)
    counter: u64,
    /// Whether the RNG has been seeded from `raw_rand()`
    seeded: bool,
}

impl CsprngState {
    /// Create initial state from IC time (weak seed, before raw_rand is available).
    fn new() -> Self {
        let time = ic_cdk::api::time();
        let mut hasher = Sha256::new();
        hasher.update(b"ic-siwa-csprng-init");
        hasher.update(time.to_le_bytes());
        let key: [u8; 32] = hasher.finalize().into();

        Self {
            key,
            counter: 0,
            seeded: false,
        }
    }

    /// Mix in strong entropy from `raw_rand()`.
    fn reseed(&mut self, entropy: &[u8]) {
        let mut hasher = Sha256::new();
        hasher.update(b"ic-siwa-csprng-reseed");
        hasher.update(&self.key);
        hasher.update(self.counter.to_le_bytes());
        hasher.update(entropy);
        self.key = hasher.finalize().into();
        self.seeded = true;
    }

    /// Generate random bytes, re-keying after each 32-byte block.
    fn fill(&mut self, dest: &mut [u8]) {
        let time = ic_cdk::api::time();

        for chunk in dest.chunks_mut(32) {
            self.counter = self.counter.wrapping_add(1);

            let mut hasher = Sha256::new();
            hasher.update(&self.key);
            hasher.update(self.counter.to_le_bytes());
            hasher.update(time.to_le_bytes());
            let output: [u8; 32] = hasher.finalize().into();

            // Copy output to destination
            chunk.copy_from_slice(&output[..chunk.len()]);

            // Re-key: derive new key from a separate hash to avoid output == key
            let mut rekey_hasher = Sha256::new();
            rekey_hasher.update(b"ic-siwa-csprng-rekey");
            rekey_hasher.update(&output);
            rekey_hasher.update(&self.key);
            self.key = rekey_hasher.finalize().into();
        }
    }
}

thread_local! {
    static RNG_STATE: RefCell<CsprngState> = RefCell::new(CsprngState::new());
}

/// Custom getrandom implementation for IC canisters.
///
/// Provides SHA-256 counter-mode CSPRNG output. The quality of the output
/// depends on whether [`seed_rng`] has been called:
///
/// - **Before seeding**: output is based on IC time only (predictable to
///   subnet nodes, suitable only for non-security-critical consumers).
/// - **After seeding**: output incorporates 32 bytes from `raw_rand()`
///   (cryptographically strong, suitable for blinding and similar uses).
fn ic_getrandom(dest: &mut [u8]) -> Result<(), Error> {
    RNG_STATE.with(|state| {
        state.borrow_mut().fill(dest);
    });
    Ok(())
}

register_custom_getrandom!(ic_getrandom);

/// Seed the CSPRNG with entropy from `raw_rand()`.
///
/// Call this once during canister init and post-upgrade to upgrade the
/// CSPRNG from time-based seeding to cryptographically strong seeding.
///
/// Safe to call multiple times — each call mixes additional entropy into
/// the existing state without discarding prior entropy.
pub async fn seed_rng() {
    match ic_cdk::management_canister::raw_rand().await {
        Ok(entropy) => {
            RNG_STATE.with(|state| {
                state.borrow_mut().reseed(&entropy);
            });
            crate::state::debug_log!("[CSPRNG] Seeded with {} bytes from raw_rand", entropy.len());
        }
        Err(e) => {
            crate::state::debug_log!(
                "[CSPRNG] WARNING: Failed to seed from raw_rand: {:?}. \
                 Falling back to time-based entropy.",
                e
            );
        }
    }
}
