//! Time utilities for IC-SIWA

/// Get current time in nanoseconds from IC
pub fn now_ns() -> u64 {
    ic_cdk::api::time()
}

/// Get current time in seconds
pub fn now_secs() -> u64 {
    now_ns() / 1_000_000_000
}

/// Check if a timestamp (in nanoseconds) has expired
pub fn is_expired_ns(timestamp_ns: u64) -> bool {
    now_ns() > timestamp_ns
}

/// Check if a timestamp (in seconds) has expired
pub fn is_expired_secs(timestamp_secs: u64) -> bool {
    now_secs() > timestamp_secs
}

/// Convert seconds to nanoseconds
pub fn secs_to_ns(secs: u64) -> u64 {
    secs * 1_000_000_000
}

/// Convert nanoseconds to seconds
pub fn ns_to_secs(ns: u64) -> u64 {
    ns / 1_000_000_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secs_to_ns() {
        assert_eq!(secs_to_ns(0), 0);
        assert_eq!(secs_to_ns(1), 1_000_000_000);
        assert_eq!(secs_to_ns(60), 60_000_000_000);
        assert_eq!(secs_to_ns(3600), 3_600_000_000_000);
    }

    #[test]
    fn test_ns_to_secs() {
        assert_eq!(ns_to_secs(0), 0);
        assert_eq!(ns_to_secs(1_000_000_000), 1);
        assert_eq!(ns_to_secs(60_000_000_000), 60);
        assert_eq!(ns_to_secs(3_600_000_000_000), 3600);
    }

    #[test]
    fn test_ns_to_secs_truncation() {
        // Should truncate, not round
        assert_eq!(ns_to_secs(1_500_000_000), 1);
        assert_eq!(ns_to_secs(1_999_999_999), 1);
        assert_eq!(ns_to_secs(999_999_999), 0);
    }

    #[test]
    fn test_round_trip() {
        let original_secs = 12345u64;
        let ns = secs_to_ns(original_secs);
        let back_to_secs = ns_to_secs(ns);
        assert_eq!(original_secs, back_to_secs);
    }
}
