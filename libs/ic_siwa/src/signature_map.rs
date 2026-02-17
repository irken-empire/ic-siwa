//! Signature map for certified delegation storage
//!
//! This module provides a `SignatureMap` that maintains a merkle tree of delegation hashes
//! required for IC canister signature authentication.

use ic_certified_map::{leaf_hash, AsHashTree, Hash, HashTree, RbTree};
use std::borrow::Cow;
use std::collections::{BinaryHeap, HashMap};

/// Minimum buffer added to delegation expiration for signature map entries (5 minutes in nanoseconds).
/// This ensures the signature remains available slightly beyond the delegation's lifetime,
/// accounting for clock skew and query latency.
pub const SIGNATURE_EXPIRATION_BUFFER_NS: u64 = 5 * 60 * 1_000_000_000;

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
    /// * `delegation_expires_at` - When the delegation itself expires (nanoseconds).
    ///   A buffer is added so the signature outlives the delegation.
    pub fn put(&mut self, seed_hash: Hash, delegation_hash: Hash, delegation_expires_at: u64) {
        let signature_expires_at =
            delegation_expires_at.saturating_add(SIGNATURE_EXPIRATION_BUFFER_NS);

        if self.certified_map.get(&seed_hash[..]).is_none() {
            let mut submap = RbTree::new();
            submap.insert(delegation_hash, Unit);
            self.certified_map.insert(seed_hash, submap);
        } else {
            self.certified_map.modify(&seed_hash[..], |submap| {
                submap.insert(delegation_hash, Unit);
            });
        }

        // Only push to queue if this is a new entry (not an update) to avoid
        // duplicate queue entries that cause overcounting and stale deletions
        let key = (seed_hash, delegation_hash);
        if !self.expiration_index.contains_key(&key) {
            self.expiration_queue.push(SigExpiration {
                seed_hash,
                delegation_hash,
                signature_expires_at,
            });
        }
        self.expiration_index.insert(key, signature_expires_at);
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

        while num_pruned < max_to_prune {
            match self.expiration_queue.peek() {
                Some(entry) if entry.signature_expires_at <= now => {}
                _ => break, // No more expired entries or queue empty
            }

            let Some(entry) = self.expiration_queue.pop() else {
                break; // Queue emptied between peek and pop (should not happen)
            };
            let key = (entry.seed_hash, entry.delegation_hash);

            // Only delete if the stored expiration matches (entry wasn't renewed)
            if let Some(&stored_exp) = self.expiration_index.get(&key) {
                if stored_exp <= now {
                    self.delete(entry.seed_hash, entry.delegation_hash);
                    num_pruned += 1;
                }
                // If stored_exp > now, the entry was renewed; skip stale queue entry
            }
            // If not in index, already deleted; skip
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
        self.expiration_index.is_empty()
    }

    /// Get the number of entries in the map
    pub fn len(&self) -> usize {
        self.expiration_index.len()
    }

    /// Get the number of entries in the expiration queue (including stale entries)
    pub fn queue_len(&self) -> usize {
        self.expiration_queue.len()
    }

    /// Remove orphaned entries from the expiration queue.
    ///
    /// When `delete()` is called, entries are removed from the certified map and
    /// expiration index but not from the `BinaryHeap` (which doesn't support
    /// arbitrary removal). This method rebuilds the queue, discarding entries
    /// that are no longer in the index.
    pub fn drain_stale(&mut self) {
        let mut new_queue = BinaryHeap::new();
        for entry in self.expiration_queue.drain() {
            let key = (entry.seed_hash, entry.delegation_hash);
            if self.expiration_index.contains_key(&key) {
                new_queue.push(entry);
            }
        }
        self.expiration_queue = new_queue;
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
        let expires_at = 1_000_000_000u64 + 30 * 60 * 1_000_000_000; // 30 min from "now"

        map.put(seed_hash, delegation_hash, expires_at);

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
        let expires_at = 1_000_000_000u64 + 30 * 60 * 1_000_000_000;

        map.put(seed_hash, delegation_hash, expires_at);
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
        // Delegation expires 30 minutes from now
        let delegation_expires_at = now + 30 * 60 * 1_000_000_000;

        map.put(seed_hash, delegation_hash, delegation_expires_at);
        assert_eq!(map.len(), 1);

        // Prune at current time - should not prune (delegation hasn't expired)
        let pruned = map.prune_expired(now, 10);
        assert_eq!(pruned, 0);
        assert_eq!(map.len(), 1);

        // Prune at delegation expiration - should not prune (buffer not yet elapsed)
        let pruned = map.prune_expired(delegation_expires_at, 10);
        assert_eq!(pruned, 0);
        assert_eq!(map.len(), 1);

        // Prune after delegation expiration + buffer
        let expired_time = delegation_expires_at + SIGNATURE_EXPIRATION_BUFFER_NS + 1;
        let pruned = map.prune_expired(expired_time, 10);
        assert_eq!(pruned, 1);
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_drain_stale_removes_orphaned_queue_entries() {
        let mut map = SignatureMap::new();
        let seed1 = make_hash(b"seed1");
        let del1 = make_hash(b"del1");
        let seed2 = make_hash(b"seed2");
        let del2 = make_hash(b"del2");
        let expires_at = 1_000_000_000u64 + 30 * 60 * 1_000_000_000;

        map.put(seed1, del1, expires_at);
        map.put(seed2, del2, expires_at);
        assert_eq!(map.len(), 2);
        assert_eq!(map.queue_len(), 2);

        // Delete one entry — removes from index/map but not from queue
        map.delete(seed1, del1);
        assert_eq!(map.len(), 1);
        assert_eq!(map.queue_len(), 2); // orphaned entry still in queue

        // Drain stale entries
        map.drain_stale();
        assert_eq!(map.queue_len(), 1); // orphaned entry removed
        assert_eq!(map.len(), 1); // active entry still present
        assert!(map.witness(seed2, del2).is_some());
    }

    #[test]
    fn test_root_hash_changes() {
        let mut map = SignatureMap::new();
        let initial_hash = map.root_hash();

        let seed_hash = make_hash(b"seed1");
        let delegation_hash = make_hash(b"delegation1");
        let expires_at = 1_000_000_000u64 + 30 * 60 * 1_000_000_000;

        map.put(seed_hash, delegation_hash, expires_at);
        let after_put_hash = map.root_hash();

        assert_ne!(initial_hash, after_put_hash);
    }
}
