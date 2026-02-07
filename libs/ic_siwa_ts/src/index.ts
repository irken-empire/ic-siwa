/**
 * IC-SIWA - Sign in with Avalanche for the Internet Computer
 *
 * This library provides client-side functionality for authenticating users
 * with their Avalanche wallets and obtaining ICP principals.
 *
 * @example
 * ```typescript
 * import { SiwaClient } from 'ic-siwa';
 *
 * const client = new SiwaClient({
 *   canisterId: 'xxxxx-xxxxx-xxxxx-xxxxx-xxx',
 * });
 *
 * // Using a viem wallet client
 * const result = await client.loginWithWallet(walletClient);
 * console.log('Logged in as:', result.principal.toText());
 *
 * // Or manually
 * const prepared = await client.prepareLogin(address);
 * const signature = await wallet.signMessage({ message: prepared.message });
 * const result = await client.login(signature, address);
 * ```
 */

export {SiwaClient, type SiwaClientOptions} from "./client";
export {SiwaError, SiwaErrorCode} from "./errors";
export {
  type SiwaIdentity,
  type SerializedIdentity,
  createSiwaIdentity,
  generateSessionKey,
} from "./identity";
export {
  type StorageProvider,
  LocalStorageProvider,
  SessionStorageProvider,
  MemoryStorageProvider,
} from "./storage";
export type {
  PreparedLogin,
  LoginResult,
  SignedDelegation,
  SiwaMessage,
  WalletClient,
  PrepareLoginOptions,
} from "./types";

// Candid types (for advanced usage)
export {
  idlFactory as siwaProviderIdlFactory,
  type _SERVICE as SiwaProviderService,
  type InitArgs as SiwaProviderInitArgs,
} from "./candid";

// Version
export const VERSION = "0.2.0";
