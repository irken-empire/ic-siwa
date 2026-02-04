/**
 * Type definitions for IC-SIWA
 */

import type {Principal} from "@dfinity/principal";

/**
 * Options for preparing a login request
 *
 * Used for multi-tenant "SIWA as a Service" where each calling application
 * can specify its own domain/uri for the wallet signing prompt.
 */
export interface PrepareLoginOptions {
  /** The Avalanche address (0x-prefixed) */
  address: string;
  /**
   * Optional domain to show in wallet (e.g., "game.tresr.community").
   * Must be whitelisted in the canister's allowed_domains.
   * Falls back to canister default if not provided.
   */
  domain?: string;
  /**
   * Optional URI to show in wallet (e.g., "https://game.tresr.community").
   * Falls back to canister default if not provided.
   */
  uri?: string;
}

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
