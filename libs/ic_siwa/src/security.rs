//! Security utilities for IC-SIWA
//!
//! Provides canister ID whitelist validation and access control guards.

use crate::error::SiwaError;
use candid::Principal;

/// Security mode for access control
#[derive(Clone, Debug, PartialEq, Default)]
pub enum SecurityMode {
    /// Development mode - permissive, logs warnings but allows all callers
    Development,
    /// Production mode - strict, rejects unauthorized callers
    #[default]
    Production,
}

/// Canister access guard for validating inter-canister calls
#[derive(Clone, Debug)]
pub struct CanisterGuard {
    /// Allowed canister principals
    allowed_canisters: Vec<Principal>,
    /// Security mode
    mode: SecurityMode,
    /// Whether to allow anonymous callers
    allow_anonymous: bool,
}

impl CanisterGuard {
    /// Create a new canister guard
    pub fn new(allowed_canisters: Vec<Principal>) -> Self {
        Self {
            allowed_canisters,
            mode: SecurityMode::Production,
            allow_anonymous: false,
        }
    }

    /// Create a guard in development mode (permissive)
    pub fn development() -> Self {
        Self {
            allowed_canisters: vec![],
            mode: SecurityMode::Development,
            allow_anonymous: true,
        }
    }

    /// Create a guard that allows any authenticated caller
    pub fn allow_all_authenticated() -> Self {
        Self {
            allowed_canisters: vec![],
            mode: SecurityMode::Production,
            allow_anonymous: false,
        }
    }

    /// Set security mode
    pub fn with_mode(mut self, mode: SecurityMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set whether to allow anonymous callers
    pub fn with_allow_anonymous(mut self, allow: bool) -> Self {
        self.allow_anonymous = allow;
        self
    }

    /// Add allowed canisters
    pub fn with_allowed_canisters(mut self, canisters: Vec<Principal>) -> Self {
        self.allowed_canisters = canisters;
        self
    }

    /// Check if the guard has any allowed canisters configured
    pub fn has_whitelist(&self) -> bool {
        !self.allowed_canisters.is_empty()
    }

    /// Validate a caller principal
    ///
    /// # Returns
    /// - `Ok(())` if the caller is allowed
    /// - `Err(SiwaError::CanisterNotAllowed)` if the caller is rejected
    pub fn validate(&self, caller: &Principal) -> Result<(), SiwaError> {
        // Check for anonymous caller
        if *caller == Principal::anonymous() {
            if self.allow_anonymous {
                return Ok(());
            }
            return Err(SiwaError::CanisterNotAllowed(
                "Anonymous callers not allowed".to_string(),
            ));
        }

        // In development mode, allow all authenticated callers
        if self.mode == SecurityMode::Development {
            // Log warning in development (would use ic_cdk::println! in canister)
            return Ok(());
        }

        // If no whitelist configured, allow all authenticated callers
        if self.allowed_canisters.is_empty() {
            return Ok(());
        }

        // Check against whitelist
        if self.allowed_canisters.contains(caller) {
            Ok(())
        } else {
            Err(SiwaError::CanisterNotAllowed(format!(
                "Canister {} not in allowed list",
                caller
            )))
        }
    }

    /// Validate the current IC caller
    ///
    /// Convenience method that gets the caller from ic_cdk
    pub fn validate_caller(&self) -> Result<Principal, SiwaError> {
        let caller = ic_cdk::api::msg_caller();
        self.validate(&caller)?;
        Ok(caller)
    }

    /// Check if a canister is in the allowed list
    pub fn is_allowed(&self, canister: &Principal) -> bool {
        if self.mode == SecurityMode::Development {
            return true;
        }
        if self.allowed_canisters.is_empty() {
            return *canister != Principal::anonymous() || self.allow_anonymous;
        }
        self.allowed_canisters.contains(canister)
    }
}

/// Controller guard for validating admin/controller access
#[derive(Clone, Debug)]
pub struct ControllerGuard {
    /// Controller principals
    controllers: Vec<Principal>,
}

impl ControllerGuard {
    /// Create a new controller guard
    pub fn new(controllers: Vec<Principal>) -> Self {
        Self { controllers }
    }

    /// Create from canister controllers (fetched at runtime)
    pub async fn from_canister_controllers() -> Result<Self, SiwaError> {
        // In a real implementation, this would fetch controllers from the management canister
        // For now, we'll use an empty list and expect it to be set explicitly
        Ok(Self {
            controllers: vec![],
        })
    }

    /// Add a controller
    pub fn with_controller(mut self, controller: Principal) -> Self {
        self.controllers.push(controller);
        self
    }

    /// Validate that the caller is a controller
    pub fn validate(&self, caller: &Principal) -> Result<(), SiwaError> {
        if *caller == Principal::anonymous() {
            return Err(SiwaError::CanisterNotAllowed(
                "Anonymous callers cannot be controllers".to_string(),
            ));
        }

        if self.controllers.is_empty() {
            // If no controllers set, reject all (fail-safe)
            return Err(SiwaError::CanisterNotAllowed(
                "No controllers configured".to_string(),
            ));
        }

        if self.controllers.contains(caller) {
            Ok(())
        } else {
            Err(SiwaError::CanisterNotAllowed(format!(
                "Caller {} is not a controller",
                caller
            )))
        }
    }

    /// Validate the current IC caller is a controller
    pub fn validate_caller(&self) -> Result<Principal, SiwaError> {
        let caller = ic_cdk::api::msg_caller();
        self.validate(&caller)?;
        Ok(caller)
    }
}

/// Macro to create a guard check at the start of a canister method
///
/// Usage in canister:
/// ```ignore
/// #[update]
/// fn protected_method() -> Result<(), String> {
///     guard_canister!(CANISTER_GUARD)?;
///     // ... method body
/// }
/// ```
#[macro_export]
macro_rules! guard_canister {
    ($guard:expr) => {
        $guard.validate_caller()
    };
}

/// Macro to require controller access
#[macro_export]
macro_rules! require_controller {
    ($guard:expr) => {
        $guard.validate_caller()
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_principal(id: u8) -> Principal {
        Principal::from_slice(&[id])
    }

    #[test]
    fn test_canister_guard_empty_whitelist() {
        let guard = CanisterGuard::new(vec![]);
        let caller = test_principal(1);

        // Empty whitelist allows all authenticated callers
        assert!(guard.validate(&caller).is_ok());
    }

    #[test]
    fn test_canister_guard_with_whitelist() {
        let allowed = test_principal(1);
        let guard = CanisterGuard::new(vec![allowed]);

        // Allowed caller
        assert!(guard.validate(&allowed).is_ok());

        // Disallowed caller
        let other = test_principal(2);
        assert!(guard.validate(&other).is_err());
    }

    #[test]
    fn test_canister_guard_anonymous() {
        let guard = CanisterGuard::new(vec![]);

        // Anonymous rejected by default
        assert!(guard.validate(&Principal::anonymous()).is_err());

        // Anonymous allowed when configured
        let guard = guard.with_allow_anonymous(true);
        assert!(guard.validate(&Principal::anonymous()).is_ok());
    }

    #[test]
    fn test_canister_guard_development_mode() {
        let guard = CanisterGuard::development();

        // Development mode allows all
        assert!(guard.validate(&test_principal(1)).is_ok());
        assert!(guard.validate(&test_principal(2)).is_ok());
        assert!(guard.validate(&Principal::anonymous()).is_ok());
    }

    #[test]
    fn test_controller_guard() {
        let controller = test_principal(1);
        let guard = ControllerGuard::new(vec![controller]);

        // Controller allowed
        assert!(guard.validate(&controller).is_ok());

        // Non-controller rejected
        let other = test_principal(2);
        assert!(guard.validate(&other).is_err());

        // Anonymous always rejected
        assert!(guard.validate(&Principal::anonymous()).is_err());
    }

    #[test]
    fn test_controller_guard_empty() {
        let guard = ControllerGuard::new(vec![]);

        // Empty controller list rejects all (fail-safe)
        assert!(guard.validate(&test_principal(1)).is_err());
    }

    #[test]
    fn test_is_allowed() {
        let allowed = test_principal(1);
        let guard = CanisterGuard::new(vec![allowed]);

        assert!(guard.is_allowed(&allowed));
        assert!(!guard.is_allowed(&test_principal(2)));
        assert!(!guard.is_allowed(&Principal::anonymous()));
    }
}
