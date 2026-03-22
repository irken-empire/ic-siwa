/**
 * SIWA Client for browser-based authentication
 */

import {
  Actor,
  HttpAgent,
  type Identity,
  type Signature,
} from "@icp-sdk/core/agent";
import {
  Delegation,
  DelegationChain,
  Ed25519KeyIdentity,
} from "@icp-sdk/core/identity";
import {Principal} from "@icp-sdk/core/principal";
import {
  idlFactory,
  type _SERVICE,
  type Result_4,
  type Result_5,
  type Result_7,
  type Delegation as CandidDelegation,
} from "./candid";
import {SiwaError, SiwaErrorCode} from "./errors";
import {
  createSiwaIdentity,
  generateSessionKey,
  type SiwaIdentity,
  type SerializedIdentity,
} from "./identity";
import {SessionStorageProvider, type StorageProvider} from "./storage";
import type {PreparedLogin, LoginResult, PrepareLoginOptions} from "./types";

/**
 * SIWA Client configuration options
 */
export interface SiwaClientOptions {
  /** Canister ID of the ic_siwa_provider */
  canisterId: string;
  /** IC host URL (default: https://ic0.app) */
  host?: string;
  /** Storage provider for caching identity (default: localStorage) */
  storage?: StorageProvider;
  /** Auto-refresh delegation before expiry */
  autoRefresh?: boolean;
  /** Refresh threshold in milliseconds (default: 5 minutes before expiry) */
  refreshThreshold?: number;
  /** Callback fired after a successful delegation refresh */
  onRefresh?: (identity: SiwaIdentity) => void;
}

/**
 * SIWA Client for authenticating with Avalanche wallets
 */
export class SiwaClient {
  private canisterId: Principal;
  private host: string;
  private storage: StorageProvider;
  private autoRefresh: boolean;
  private refreshThreshold: number;
  private onRefresh?: (identity: SiwaIdentity) => void;
  private identity: SiwaIdentity | null = null;
  private agent: HttpAgent | null = null;
  private refreshTimer: ReturnType<typeof setTimeout> | null = null;
  private sessionKey: Ed25519KeyIdentity | null = null;
  private pendingLogin: Promise<LoginResult> | null = null;

  constructor(options: SiwaClientOptions) {
    this.canisterId = Principal.fromText(options.canisterId);
    this.host = options.host ?? "https://ic0.app";
    this.storage = options.storage ?? new SessionStorageProvider();
    this.autoRefresh = options.autoRefresh ?? true;
    this.refreshThreshold = options.refreshThreshold ?? 5 * 60 * 1000; // 5 minutes
    this.onRefresh = options.onRefresh;
  }

  /**
   * Get a canister-namespaced storage key to prevent cross-canister conflicts
   */
  private storageKey(key: string): string {
    return `${this.canisterId.toText()}_${key}`;
  }

  /**
   * Check if user is currently authenticated
   */
  async isAuthenticated(): Promise<boolean> {
    const identity = await this.getIdentity();
    return identity !== null && !identity.isExpired();
  }

  /**
   * Get the current identity if authenticated
   */
  async getIdentity(): Promise<SiwaIdentity | null> {
    if (this.identity && !this.identity.isExpired()) {
      return this.identity;
    }

    // Try to restore from storage
    const stored = await this.storage.get(this.storageKey("identity"));
    if (stored) {
      try {
        const data = JSON.parse(stored) as SerializedIdentity;
        this.identity = await createSiwaIdentity(data);

        // Use a 30-second buffer so we don't return a nearly-expired
        // identity that would fail on the first canister call.
        const RESTORE_BUFFER_MS = 30_000;
        if (Date.now() + RESTORE_BUFFER_MS < this.identity.getExpiration()) {
          // Restore session key if stored
          const storedKey = await this.storage.get(
            this.storageKey("session_key")
          );
          if (storedKey) {
            this.sessionKey = Ed25519KeyIdentity.fromJSON(storedKey);
          }

          // Set up auto-refresh if enabled
          if (this.autoRefresh) {
            this.scheduleRefresh();
          }

          return this.identity;
        }
      } catch {
        // Invalid stored identity, clean up
      }
      await this.storage.remove(this.storageKey("identity"));
      await this.storage.remove(this.storageKey("session_key"));
    }

    return null;
  }

  /**
   * Get the current principal if authenticated
   */
  async getPrincipal(): Promise<Principal | null> {
    const identity = await this.getIdentity();
    return identity?.getPrincipal() ?? null;
  }

  /**
   * Get the current Avalanche address if authenticated
   */
  async getAddress(): Promise<string | null> {
    const identity = await this.getIdentity();
    return identity?.getAddress() ?? null;
  }

  /**
   * Check if the host is a local development environment.
   * Uses strict hostname matching to avoid false positives
   * (e.g. "localhost.evil.com" would NOT match).
   */
  private isLocalEnvironment(): boolean {
    try {
      const url = new URL(this.host);
      const hostname = url.hostname;
      return (
        hostname === "localhost" ||
        hostname === "127.0.0.1" ||
        hostname === "::1" ||
        hostname.endsWith(".localhost")
      );
    } catch {
      return false;
    }
  }

  /**
   * Validate an Avalanche address format (0x-prefixed, 42 hex chars)
   */
  private static validateAddress(address: string): void {
    if (!/^0x[0-9a-fA-F]{40}$/.test(address)) {
      throw new SiwaError(
        SiwaErrorCode.InvalidAddress,
        "Address must be 0x-prefixed followed by 40 hex characters"
      );
    }
  }

  /**
   * Validate a hex-encoded signature format (0x-prefixed, 130 hex chars = 65 bytes)
   */
  private static validateSignature(signature: string): void {
    if (!/^0x[0-9a-fA-F]{130}$/.test(signature)) {
      throw new SiwaError(
        SiwaErrorCode.InvalidSignature,
        "Signature must be 0x-prefixed followed by 130 hex characters (65 bytes)"
      );
    }
  }

  /**
   * Create an anonymous agent for canister calls
   */
  private async createAnonymousAgent(): Promise<HttpAgent> {
    const agent = await HttpAgent.create({
      host: this.host,
    });

    if (this.isLocalEnvironment()) {
      await agent.fetchRootKey();
    }

    return agent;
  }

  /**
   * Create an actor for the SIWA provider canister
   */
  private async createProviderActor(agent?: HttpAgent): Promise<_SERVICE> {
    const actorAgent = agent ?? (await this.createAnonymousAgent());
    return Actor.createActor<_SERVICE>(idlFactory, {
      agent: actorAgent,
      canisterId: this.canisterId,
    });
  }

  /**
   * Prepare a login message for signing (simple version)
   *
   * Uses the canister's default domain/uri. For multi-tenant scenarios,
   * use `prepareLoginWithOptions` to specify your app's domain.
   *
   * @param address - The Avalanche address (0x-prefixed)
   */
  async prepareLogin(address: string): Promise<PreparedLogin> {
    return this.prepareLoginWithOptions({address});
  }

  /**
   * Prepare a login message with custom domain/uri (multi-tenant version)
   *
   * This is the "SIWA as a Service" method where each calling application
   * can specify its own domain/uri for the wallet signing prompt.
   *
   * @param options - Login options including address and optional domain/uri
   * @param options.address - The Avalanche address (0x-prefixed)
   * @param options.domain - Domain to show in wallet (must be whitelisted)
   * @param options.uri - URI to show in wallet
   *
   * @example
   * ```ts
   * const prepared = await client.prepareLoginWithOptions({
   *   address: "0x1234...",
   *   domain: "game.tresr.community",
   *   uri: "https://game.tresr.community"
   * });
   * ```
   */
  async prepareLoginWithOptions(
    options: PrepareLoginOptions
  ): Promise<PreparedLogin> {
    SiwaClient.validateAddress(options.address);

    // Client-side domain validation
    if (options.domain !== undefined) {
      if (typeof options.domain !== "string" || options.domain.length === 0) {
        throw new SiwaError(
          SiwaErrorCode.InvalidInput,
          "Domain must be a non-empty string"
        );
      }
    }

    // Client-side URI validation
    if (options.uri !== undefined) {
      if (typeof options.uri !== "string" || options.uri.length === 0) {
        throw new SiwaError(
          SiwaErrorCode.InvalidInput,
          "URI must be a non-empty string"
        );
      }
      try {
        new URL(options.uri);
      } catch {
        throw new SiwaError(
          SiwaErrorCode.InvalidInput,
          `Invalid URI format: ${options.uri}`
        );
      }
    }

    try {
      const actor = await this.createProviderActor();

      // Use the new multi-tenant endpoint if domain/uri provided
      const response: Result_7 = await actor.siwa_prepare_login_with_options({
        address: options.address,
        domain: options.domain ? [options.domain] : [],
        uri: options.uri ? [options.uri] : [],
      });

      if ("Err" in response) {
        throw SiwaError.fromCanisterError(response.Err);
      }

      // Response.Ok is a PrepareLoginResponse record with message, nonce, expiration
      const {message, nonce, expiration} = response.Ok;

      return {
        message,
        nonce,
        expiration,
      };
    } catch (error) {
      if (error instanceof SiwaError) {
        throw error;
      }
      throw SiwaError.fromCanisterError(error);
    }
  }

  /**
   * Complete login with signed message
   *
   * @param signature - The signed message from the wallet
   * @param address - The Avalanche address
   * @param sessionKey - Optional session key (generated if not provided)
   */
  async login(
    signature: string,
    address: string,
    sessionKey?: Ed25519KeyIdentity
  ): Promise<LoginResult> {
    SiwaClient.validateAddress(address);
    SiwaClient.validateSignature(signature);
    try {
      // Generate session key if not provided
      this.sessionKey = sessionKey ?? generateSessionKey();
      const sessionKeyDer = this.sessionKey.getPublicKey().toDer();
      // Convert to number[] for Candid encoding (blob = vec nat8)
      const sessionKeyBytes = Array.from(new Uint8Array(sessionKeyDer));

      // Combined login + prepare_delegation in a single update call
      // This saves ~2 seconds by eliminating one consensus round-trip
      const actor = await this.createProviderActor();
      const loginResponse: Result_5 = await actor.siwa_login_and_prepare(
        signature,
        address,
        sessionKeyBytes
      );

      if ("Err" in loginResponse) {
        throw SiwaError.fromCanisterError(loginResponse.Err);
      }

      // LoginResponse contains user_principal, expiration, and user_canister_pubkey
      const {
        user_principal: principal,
        expiration: loginExpiration,
        user_canister_pubkey: userCanisterPubkey,
      } = loginResponse.Ok;

      // Convert the canister's public key to Uint8Array
      // This is the ROOT of the delegation chain (the signer)
      const canisterPubkeyBytes =
        userCanisterPubkey instanceof Uint8Array
          ? userCanisterPubkey
          : new Uint8Array(userCanisterPubkey);

      // Use the expiration from login response (always present in LoginResponse)
      const expirationNs = loginExpiration;

      // Get the certified delegation with retry logic to handle the IC
      // certified data propagation race (query may hit a replica that hasn't
      // yet processed the update call's state tree commit).
      const delegationResponse = await this.getDelegationWithRetry(
        actor,
        address,
        sessionKeyBytes,
        expirationNs
      );

      // Create delegation chain from canister response
      const {delegation: candidDelegation, signature: delegationSignature} =
        delegationResponse.Ok;

      // Convert signature to proper Uint8Array if needed
      const signatureBytes =
        delegationSignature instanceof Uint8Array
          ? delegationSignature
          : new Uint8Array(delegationSignature);

      // Build delegation chain from the canister's delegation
      // The canister's public key is the root (signer) of the delegation chain
      const delegationChain = this.buildDelegationChain(
        candidDelegation,
        signatureBytes,
        canisterPubkeyBytes
      );

      // Calculate expiration in milliseconds
      const expirationMs = Number(expirationNs / BigInt(1_000_000));

      // Convert canister pubkey to hex for storage
      const canisterPubkeyHex = Array.from(canisterPubkeyBytes)
        .map((b) => b.toString(16).padStart(2, "0"))
        .join("");

      // Create serialized identity data
      const serializedIdentity: SerializedIdentity = {
        baseKey: JSON.stringify(this.sessionKey.toJSON()),
        delegationChain: JSON.stringify(delegationChain.toJSON()),
        expiration: expirationMs,
        address,
        canisterPubkey: canisterPubkeyHex,
      };

      // Create identity
      this.identity = await createSiwaIdentity(serializedIdentity);

      // Store identity and session key
      await this.storage.set(
        this.storageKey("identity"),
        JSON.stringify(serializedIdentity)
      );
      await this.storage.set(
        this.storageKey("session_key"),
        JSON.stringify(this.sessionKey.toJSON())
      );

      // Reset agent to use new identity
      this.agent = null;

      // Set up auto-refresh if enabled
      if (this.autoRefresh) {
        this.scheduleRefresh();
      }

      return {
        principal,
        address,
        expiration: expirationMs,
      };
    } catch (error) {
      if (error instanceof SiwaError) {
        throw error;
      }
      throw SiwaError.fromCanisterError(error);
    }
  }

  /**
   * Get delegation with retry logic to handle IC certified data propagation delay.
   *
   * After an update call modifies certified data, a subsequent query may hit a
   * replica that hasn't yet processed the new block. This retries with backoff.
   */
  private async getDelegationWithRetry(
    actor: _SERVICE,
    address: string,
    sessionKeyBytes: number[],
    expirationNs: bigint,
    maxRetries = 3,
    baseDelayMs = 500
  ): Promise<Extract<Result_4, {Ok: unknown}>> {
    let lastError: string | undefined;
    for (let attempt = 0; attempt < maxRetries; attempt++) {
      const response: Result_4 = await actor.siwa_get_delegation(
        address,
        sessionKeyBytes,
        expirationNs
      );
      if ("Ok" in response) {
        return response as Extract<Result_4, {Ok: unknown}>;
      }
      lastError = response.Err;
      // Wait with linear backoff before retrying
      await new Promise((resolve) =>
        setTimeout(resolve, baseDelayMs * (attempt + 1))
      );
    }
    throw SiwaError.fromCanisterError(
      lastError ?? "Failed to get delegation after retries"
    );
  }

  /**
   * Build delegation chain from canister response
   *
   * The delegation chain structure is:
   * - Root (publicKey parameter): The canister's derived public key for this user
   * - Delegation: Grants the session key the ability to act on behalf of the user
   *
   * @param candidDelegation - Delegation record from canister (contains session key as pubkey)
   * @param signatureBytes - Signature from canister over the delegation
   * @param canisterPubkey - The canister's public key (root of the chain, the signer)
   */
  private buildDelegationChain(
    candidDelegation: CandidDelegation,
    signatureBytes: Uint8Array,
    canisterPubkey: Uint8Array
  ): DelegationChain {
    if (!this.sessionKey) {
      throw new SiwaError(
        SiwaErrorCode.NotAuthenticated,
        "No session key available"
      );
    }

    // The delegation's pubkey is the session key - the key being delegated TO
    // @icp-sdk/core/identity uses Uint8Array instead of ArrayBuffer
    const sessionKeyPubkey =
      candidDelegation.pubkey instanceof Uint8Array
        ? candidDelegation.pubkey
        : new Uint8Array(candidDelegation.pubkey);

    // Create Delegation instance from canister response
    // Note: targets are optional in the candid type
    const targets = candidDelegation.targets[0]; // opt vec principal -> [] | [Principal[]]
    const delegation = new Delegation(
      sessionKeyPubkey,
      candidDelegation.expiration,
      targets
    );

    // Create delegation chain:
    // - delegations[0]: canister delegates to session key
    // - publicKey: canister's public key (the root/signer of the chain)
    //
    // This allows the session key to sign requests that will be verified
    // as coming from the user's canister-derived identity.
    return DelegationChain.fromDelegations(
      [
        {
          delegation,
          signature: signatureBytes as Signature,
        },
      ],
      canisterPubkey as unknown as import("@icp-sdk/core/agent").DerEncodedPublicKey
    );
  }

  /**
   * Refresh the current delegation
   */
  async refreshDelegation(): Promise<LoginResult | null> {
    const identity = await this.getIdentity();
    if (!identity || !this.sessionKey) {
      return null;
    }

    const address = identity.getAddress();
    // Convert to number[] for Candid encoding (blob = vec nat8)
    const sessionKeyBytes = Array.from(
      new Uint8Array(this.sessionKey.getPublicKey().toDer())
    );

    // Get the stored canister pubkey from the serialized identity
    const serialized = identity.serialize();
    if (!serialized.canisterPubkey) {
      // Legacy identity without canister pubkey, need to re-login
      await this.logout();
      return null;
    }

    // Convert hex back to Uint8Array
    const hexPairs = serialized.canisterPubkey.match(/.{1,2}/g);
    if (!hexPairs || hexPairs.length === 0) {
      throw new Error("Invalid hex-encoded canister public key");
    }
    const canisterPubkeyBytes = new Uint8Array(
      hexPairs.map((b) => parseInt(b, 16))
    );

    try {
      const actor = await this.createProviderActor();

      // Request the maximum possible expiration and let the canister cap it
      // to its configured session_expiration_time. Using the old (nearly expired)
      // delegation's timestamp would produce an immediately-expiring delegation.
      const requestedExpirationNs =
        BigInt(Number.MAX_SAFE_INTEGER) * BigInt(1_000_000);

      // Prepare delegation first (stores in signature map for certified response)
      const prepareResult = await actor.siwa_prepare_delegation(
        address,
        sessionKeyBytes,
        requestedExpirationNs
      );

      if ("Err" in prepareResult) {
        // Session may have expired, need to re-login
        await this.logout();
        return null;
      }

      // Use the canister's capped expiration (not the uncapped requested value)
      // to ensure the expiration passed to siwa_get_delegation matches what was
      // actually stored during prepare_delegation.
      const cappedExpirationNs = prepareResult.Ok;

      // Get the certified delegation with retry logic
      let delegationResponse;
      try {
        delegationResponse = await this.getDelegationWithRetry(
          actor,
          address,
          sessionKeyBytes,
          cappedExpirationNs
        );
      } catch {
        // Session may have expired or delegation unavailable, need to re-login
        await this.logout();
        return null;
      }

      const {delegation: candidDelegation, signature: delegationSignature} =
        delegationResponse.Ok;

      const signatureBytes =
        delegationSignature instanceof Uint8Array
          ? delegationSignature
          : new Uint8Array(delegationSignature);

      const delegationChain = this.buildDelegationChain(
        candidDelegation,
        signatureBytes,
        canisterPubkeyBytes
      );

      // Use the canister's actual expiration from the delegation response
      const expirationMs = Number(
        candidDelegation.expiration / BigInt(1_000_000)
      );

      const serializedIdentity: SerializedIdentity = {
        baseKey: JSON.stringify(this.sessionKey.toJSON()),
        delegationChain: JSON.stringify(delegationChain.toJSON()),
        expiration: expirationMs,
        address,
        canisterPubkey: serialized.canisterPubkey,
      };

      this.identity = await createSiwaIdentity(serializedIdentity);
      await this.storage.set(
        this.storageKey("identity"),
        JSON.stringify(serializedIdentity)
      );

      // Reset agent
      this.agent = null;

      // Reschedule refresh
      if (this.autoRefresh) {
        this.scheduleRefresh();
      }

      // Notify consumer of the refreshed identity
      if (this.onRefresh && this.identity) {
        try {
          this.onRefresh(this.identity);
        } catch {
          // Don't let callback errors break the refresh flow
        }
      }

      return {
        principal: this.identity.getPrincipal(),
        address,
        expiration: expirationMs,
      };
    } catch {
      // Failed to refresh, clear state
      await this.logout();
      return null;
    }
  }

  /**
   * Schedule automatic delegation refresh
   */
  private scheduleRefresh(): void {
    // Clear existing timer
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer);
      this.refreshTimer = null;
    }

    if (!this.identity) {
      return;
    }

    const expiration = this.identity.getExpiration();
    const timeUntilRefresh = expiration - Date.now() - this.refreshThreshold;

    if (timeUntilRefresh > 0) {
      // Clamp to 32-bit signed max: setTimeout uses a 32-bit int internally,
      // so delays > ~24.8 days (2^31 - 1 ms) overflow and fire immediately.
      const MAX_TIMEOUT = 2_147_483_647;
      this.refreshTimer = setTimeout(
        () => {
          this.refreshDelegation().catch(() => {
            // Refresh failed, will be handled on next auth check
          });
        },
        Math.min(timeUntilRefresh, MAX_TIMEOUT)
      );
    }
  }

  /**
   * Login with a wallet client (convenience method)
   *
   * @param walletClient - A viem wallet client or similar
   * @param options - Optional domain/uri for multi-tenant scenarios
   * @param options.domain - Domain to show in wallet (must be whitelisted)
   * @param options.uri - URI to show in wallet
   *
   * @example
   * ```ts
   * // Simple usage (uses canister defaults)
   * await client.loginWithWallet(walletClient);
   *
   * // Multi-tenant usage (specify your app's domain)
   * await client.loginWithWallet(walletClient, {
   *   domain: "game.tresr.community",
   *   uri: "https://game.tresr.community"
   * });
   * ```
   */
  async loginWithWallet(
    walletClient: {
      account: {address: string};
      signMessage: (args: {message: string}) => Promise<string>;
    },
    options?: {domain?: string; uri?: string}
  ): Promise<LoginResult> {
    // If a login is already in progress, return the pending result
    // to prevent duplicate delegation requests from concurrent callers
    if (this.pendingLogin) {
      return this.pendingLogin;
    }

    const doLogin = async (): Promise<LoginResult> => {
      const address = walletClient.account.address;

      // Prepare login message with optional domain/uri
      const prepared = await this.prepareLoginWithOptions({
        address,
        domain: options?.domain,
        uri: options?.uri,
      });

      // Sign message with wallet
      const signature = await walletClient.signMessage({
        message: prepared.message,
      });

      // Complete login
      return this.login(signature, address);
    };

    this.pendingLogin = doLogin().finally(() => {
      this.pendingLogin = null;
    });

    return this.pendingLogin;
  }

  /**
   * Logout and clear stored identity
   *
   * Revokes the session on the canister (best-effort) before clearing
   * local state. If the canister is unreachable, local logout still
   * succeeds and the server-side session will expire naturally.
   */
  async logout(): Promise<void> {
    // Clear refresh timer
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer);
      this.refreshTimer = null;
    }

    // Revoke session on the canister (best-effort, don't block on failure).
    // Uses the authenticated agent so the caller principal matches the session
    // owner, which is required by the canister's authorization check.
    if (this.identity && this.sessionKey) {
      try {
        const address = this.identity.getAddress();
        const sessionKeyBytes = Array.from(
          new Uint8Array(this.sessionKey.getPublicKey().toDer())
        );
        const agent = await this.getAgent();
        const actor = await this.createProviderActor(agent);
        await actor.siwa_logout(address, sessionKeyBytes);
      } catch {
        // Best-effort: canister may be unreachable or delegation already expired,
        // session will expire naturally on the canister side
      }
    }

    this.identity = null;
    this.agent = null;
    this.sessionKey = null;

    await this.storage.remove(this.storageKey("identity"));
    await this.storage.remove(this.storageKey("session_key"));
  }

  /**
   * Get an HttpAgent for making authenticated calls
   */
  async getAgent(): Promise<HttpAgent> {
    const identity = await this.getIdentity();
    if (!identity) {
      throw new SiwaError(SiwaErrorCode.NotAuthenticated, "Not authenticated");
    }

    if (!this.agent) {
      this.agent = await HttpAgent.create({
        host: this.host,
        identity: identity.getDelegationIdentity() as unknown as Identity,
      });

      if (this.isLocalEnvironment()) {
        await this.agent.fetchRootKey();
      }
    }

    return this.agent;
  }

  /**
   * Create an actor for calling a canister
   */
  async createActor<T>(
    canisterId: string | Principal,
    idlFactory: unknown
  ): Promise<T> {
    const agent = await this.getAgent();
    return Actor.createActor(idlFactory as never, {
      agent,
      canisterId:
        typeof canisterId === "string"
          ? Principal.fromText(canisterId)
          : canisterId,
    }) as T;
  }

  /**
   * Get the canister ID
   */
  getCanisterId(): Principal {
    return this.canisterId;
  }

  /**
   * Get the host URL
   */
  getHost(): string {
    return this.host;
  }
}
