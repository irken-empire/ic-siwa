/**
 * Type definitions for IC-SIWA
 */

import type {Principal} from "@dfinity/principal";

/**
 * Prepared login response from canister
 */
export interface PreparedLogin {
  /** SIWA message to sign */
  message: string;
  /** Nonce used in message */
  nonce: string;
  /** Expiration timestamp (nanoseconds) */
  expiration: bigint;
}

/**
 * Login result
 */
export interface LoginResult {
  /** ICP principal for the authenticated user */
  principal: Principal;
  /** Avalanche address */
  address: string;
  /** Session expiration timestamp (ms) */
  expiration: number;
}

/**
 * Signed delegation from canister
 */
export interface SignedDelegation {
  /** Delegation bytes */
  delegation: Uint8Array;
  /** Signature bytes */
  signature: Uint8Array;
}

/**
 * SIWA message structure (EIP-4361 compatible)
 */
export interface SiwaMessage {
  /** Domain requesting the sign-in */
  domain: string;
  /** Avalanche address */
  address: string;
  /** Human-readable statement */
  statement?: string;
  /** URI of the requesting application */
  uri: string;
  /** Current version of the message */
  version: string;
  /** Chain ID (Avalanche C-Chain) */
  chainId: number;
  /** Randomized token for preventing replay attacks */
  nonce: string;
  /** Timestamp of message creation (ISO 8601) */
  issuedAt: string;
  /** Timestamp when message expires (ISO 8601) */
  expirationTime?: string;
  /** Timestamp when message becomes valid (ISO 8601) */
  notBefore?: string;
  /** System-specific request ID */
  requestId?: string;
  /** List of resources the user is requesting access to */
  resources?: string[];
}

/**
 * Wallet client interface (compatible with viem)
 */
export interface WalletClient {
  /** Connected account */
  account: {
    /** Avalanche address (0x prefixed) */
    address: string;
  };
  /** Sign a message */
  signMessage(args: {message: string}): Promise<string>;
}
