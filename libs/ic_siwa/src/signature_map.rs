//! Signature map for certified delegation storage
//!
//! This module provides a `SignatureMap` that maintains a merkle tree of delegation hashes
//! required for IC canister signature authentication.

use ic_certified_map::{leaf_hash, AsHashTree, Hash, HashTree, RbTree};
use std::borrow::Cow;
use std::collections::{BinaryHeap, HashMap};

/// Default expiration time for delegation signatures (1 minute in nanoseconds)
pub const DELEGATION_SIGNATURE_EXPIRES_AT: u64 = 60 * 1_000_000_000;

/// Unit type for the inner tree values
#[derive(Default)]
struct Unit;

impl AsHashTree for Unit {
    fn root_hash(&self) -> Hash {
        leaf_hash(&b""[..])
    }
    fn as_hash_tree(&self) -> HashTree<'_> {
        HashTree::Leaf(Cow::from(&b""[..]))
    }
}

/// Expiration entry for tracking when signatures should be pruned
#[derive(PartialEq, Eq)]
struct SigExpiration {
    seed_hash: Hash,
    delegation_hash: Hash,
    signature_expires_at: u64,
}

impl Ord for SigExpiration {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap is a max heap, but we want expired entries
        // first, hence the inversed order.
        other.signature_expires_at.cmp(&self.signature_expires_at)
    }
}

impl PartialOrd for SigExpiration {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// The SignatureMap maintains the tree of delegation hashes required for authentication.
///
/// This is used to create certified data that the IC can verify. When a delegation is
/// created during login, its hash is stored in this map. When `siwa_get_delegation` is
/// called, the map provides a witness (merkle proof) that the delegation exists.
#[derive(Default)]
pub struct SignatureMap {
    certified_map: RbTree<Hash, RbTree<Hash, Unit>>,
    expiration_queue: BinaryHeap<SigExpiration>,
    /// O(1) lookup index: (seed_hash, delegation_hash) -> expiration timestamp
    expiration_index: HashMap<(Hash, Hash), u64>,
}

impl SignatureMap {
    /// Create a new empty SignatureMap
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a delegation hash to the map
    ///
    /// # Arguments
    /// * `seed_hash` - Hash of the seed (derived from address + salt)
    /// * `delegation_hash` - Hash of the delegation
    /// * `now` - Current timestamp in nanoseconds
    pub fn put(&mut self, seed_hash: Hash, delegation_hash: Hash, now: u64) {
        let signature_expires_at = now.saturating_add(DELEGATION_SIGNATURE_EXPIRES_AT);

        if self.certified_map.get(&seed_hash[..]).is_none() {
            let mut submap = RbTree::new();
            submap.insert(delegation_hash, Unit);
            self.certified_map.insert(seed_hash, submap);
        } else {
            self.certified_map.modify(&seed_hash[..], |submap| {
                submap.insert(delegation_hash, Unit);
            });
        }

        self.expiration_queue.push(SigExpiration {
            seed_hash,
            delegation_hash,
            signature_expires_at,
        });
        self.expiration_index
            .insert((seed_hash, delegation_hash), signature_expires_at);
    }

    /// Remove a delegation hash from the map
    pub fn delete(&mut self, seed_hash: Hash, delegation_hash: Hash) {
        let mut is_empty = false;
        self.certified_map.modify(&seed_hash[..], |m| {
            m.delete(&delegation_hash[..]);
            is_empty = m.is_empty();
        });
        if is_empty {
            self.certified_map.delete(&seed_hash[..]);
        }
        self.expiration_index.remove(&(seed_hash, delegation_hash));
    }

    /// Prune expired entries from the map
    ///
    /// # Arguments
    /// * `now` - Current timestamp in nanoseconds
    /// * `max_to_prune` - Maximum number of entries to prune
    ///
    /// # Returns
    /// Number of entries pruned
    pub fn prune_expired(&mut self, now: u64, max_to_prune: usize) -> usize {
        let mut num_pruned = 0;
        let max_to_prune = std::cmp::min(max_to_prune, self.expiration_queue.len());

        for _ in 0..max_to_prune {
            if let Some(expiration) = self.expiration_queue.peek() {
                if expiration.signature_expires_at > now {
                    return num_pruned;
                }
            }
            if let Some(expiration) = self.expiration_queue.pop() {
                self.delete(expiration.seed_hash, expiration.delegation_hash);
            }
            num_pruned += 1;
        }

        num_pruned
    }

    /// Check if a delegation signature has expired
    ///
    /// Returns true if:
    /// - The delegation is in the expiration queue and has expired
    /// - The delegation is NOT in the certified map (regardless of expiration queue)
    ///
    /// Returns false if the delegation exists in the certified map and hasn't expired
    pub fn is_expired(&self, now: u64, seed_hash: Hash, delegation_hash: Hash) -> bool {
        // First check if it exists in the certified map
        let exists_in_map = self
            .certified_map
            .get(&seed_hash[..])
            .and_then(|inner| inner.get(&delegation_hash[..]))
            .is_some();

        if !exists_in_map {
            // Not in the certified map means it's effectively expired/pruned
            return true;
        }

        // O(1) lookup of expiration time via index
        if let Some(&expires_at) = self.expiration_index.get(&(seed_hash, delegation_hash)) {
            return now > expires_at;
        }

        // Exists in certified map but not in expiration index - consider valid
        // (This shouldn't happen normally, but better to allow than block)
        false
    }

    /// Get the root hash of the certified map
    ///
    /// This hash should be set as the canister's certified data via
    /// `ic_cdk::api::set_certified_data()`
    pub fn root_hash(&self) -> Hash {
        self.certified_map.root_hash()
    }

    /// Create a witness (merkle proof) for a delegation
    ///
    /// # Arguments
    /// * `seed_hash` - Hash of the seed
    /// * `delegation_hash` - Hash of the delegation
    ///
    /// # Returns
    /// A HashTree witness if the delegation exists, None otherwise
    pub fn witness(&self, seed_hash: Hash, delegation_hash: Hash) -> Option<HashTree<'_>> {
        // Check if the entry exists
        self.certified_map
            .get(&seed_hash[..])?
            .get(&delegation_hash[..])?;

        // Create nested witness
        let witness = self.certified_map.nested_witness(&seed_hash[..], |nested| {
            nested.witness(&delegation_hash[..])
        });
        Some(witness)
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.expiration_queue.is_empty()
    }

    /// Get the number of entries in the map
    pub fn len(&self) -> usize {
        self.expiration_queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::sha256;

    fn make_hash(data: &[u8]) -> Hash {
        sha256(data)
    }

    #[test]
    fn test_put_and_witness() {
        let mut map = SignatureMap::new();
        let seed_hash = make_hash(b"seed1");
        let delegation_hash = make_hash(b"delegation1");
        let now = 1_000_000_000u64;

        map.put(seed_hash, delegation_hash, now);

        let witness = map.witness(seed_hash, delegation_hash);
        assert!(witness.is_some());
    }

    #[test]
    fn test_witness_nonexistent() {
        let map = SignatureMap::new();
        let seed_hash = make_hash(b"seed1");
        let delegation_hash = make_hash(b"delegation1");

        let witness = map.witness(seed_hash, delegation_hash);
        assert!(witness.is_none());
    }

    #[test]
    fn test_delete() {
        let mut map = SignatureMap::new();
        let seed_hash = make_hash(b"seed1");
        let delegation_hash = make_hash(b"delegation1");
        let now = 1_000_000_000u64;

        map.put(seed_hash, delegation_hash, now);
        assert!(map.witness(seed_hash, delegation_hash).is_some());

        map.delete(seed_hash, delegation_hash);
        assert!(map.witness(seed_hash, delegation_hash).is_none());
    }

    #[test]
    fn test_prune_expired() {
        let mut map = SignatureMap::new();
        let seed_hash = make_hash(b"seed1");
        let delegation_hash = make_hash(b"delegation1");
        let now = 1_000_000_000u64;

        map.put(seed_hash, delegation_hash, now);
        assert_eq!(map.len(), 1);

        // Prune with current time - should not prune
        let pruned = map.prune_expired(now, 10);
        assert_eq!(pruned, 0);
        assert_eq!(map.len(), 1);

        // Prune after expiration
        let expired_time = now + DELEGATION_SIGNATURE_EXPIRES_AT + 1;
        let pruned = map.prune_expired(expired_time, 10);
        assert_eq!(pruned, 1);
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_root_hash_changes() {
        let mut map = SignatureMap::new();
        let initial_hash = map.root_hash();

        let seed_hash = make_hash(b"seed1");
        let delegation_hash = make_hash(b"delegation1");
        let now = 1_000_000_000u64;

        map.put(seed_hash, delegation_hash, now);
        let after_put_hash = map.root_hash();

        assert_ne!(initial_hash, after_put_hash);
    }
}
