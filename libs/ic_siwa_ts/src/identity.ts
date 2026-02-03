/**
 * SIWA Identity for authenticated sessions
 */

import {
  DelegationIdentity,
  DelegationChain,
  Ed25519KeyIdentity,
} from "@dfinity/identity";
import {Principal} from "@dfinity/principal";

/**
 * Serialized identity data for storage
 */
export interface SerializedIdentity {
  /** Base key (Ed25519) in hex */
  baseKey: string;
  /** Delegation chain JSON */
  delegationChain: string;
  /** Expiration timestamp (ms) */
  expiration: number;
  /** Avalanche address */
  address: string;
}

/**
 * SIWA Identity wrapper
 */
export interface SiwaIdentity {
  /** Get the ICP principal */
  getPrincipal(): Principal;
  /** Get the Avalanche address */
  getAddress(): string;
  /** Check if the identity is expired */
  isExpired(): boolean;
  /** Get expiration timestamp */
  getExpiration(): number;
  /** Serialize for storage */
  serialize(): SerializedIdentity;
  /** Get the underlying delegation identity */
  getDelegationIdentity(): DelegationIdentity;
}

/**
 * Create a SIWA identity from serialized data
 */
export async function createSiwaIdentity(
  data: SerializedIdentity
): Promise<SiwaIdentity> {
  // Restore base key
  const baseKey = Ed25519KeyIdentity.fromJSON(data.baseKey);

  // Restore delegation chain
  const delegationChain = DelegationChain.fromJSON(data.delegationChain);

  // Create delegation identity
  const delegationIdentity = DelegationIdentity.fromDelegation(
    baseKey,
    delegationChain
  );

  return {
    getPrincipal(): Principal {
      return delegationIdentity.getPrincipal();
    },

    getAddress(): string {
      return data.address;
    },

    isExpired(): boolean {
      return Date.now() > data.expiration;
    },

    getExpiration(): number {
      return data.expiration;
    },

    serialize(): SerializedIdentity {
      return data;
    },

    getDelegationIdentity(): DelegationIdentity {
      return delegationIdentity;
    },
  };
}

/**
 * Generate a new session key pair
 */
export function generateSessionKey(): Ed25519KeyIdentity {
  return Ed25519KeyIdentity.generate();
}
