/**
 * SIWA Client for browser-based authentication
 */

import {Actor, HttpAgent, type Identity, type Signature} from "@dfinity/agent";
import {
  Delegation,
  DelegationChain,
  Ed25519KeyIdentity,
} from "@dfinity/identity";
import {Principal} from "@dfinity/principal";
import {
  idlFactory,
  type _SERVICE,
  type Result_3,
  type Result_4,
  type Result_5,
  type Delegation as CandidDelegation,
} from "./candid";
import {SiwaError, SiwaErrorCode} from "./errors";
import {
  createSiwaIdentity,
  generateSessionKey,
  type SiwaIdentity,
  type SerializedIdentity,
} from "./identity";
import {LocalStorageProvider, type StorageProvider} from "./storage";
import type {PreparedLogin, LoginResult} from "./types";

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
  private identity: SiwaIdentity | null = null;
  private agent: HttpAgent | null = null;
  private refreshTimer: ReturnType<typeof setTimeout> | null = null;
  private sessionKey: Ed25519KeyIdentity | null = null;

  constructor(options: SiwaClientOptions) {
    this.canisterId = Principal.fromText(options.canisterId);
    this.host = options.host ?? "https://ic0.app";
    this.storage = options.storage ?? new LocalStorageProvider();
    this.autoRefresh = options.autoRefresh ?? true;
    this.refreshThreshold = options.refreshThreshold ?? 5 * 60 * 1000; // 5 minutes
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
    const stored = await this.storage.get("siwa_identity");
    if (stored) {
      try {
        const data = JSON.parse(stored) as SerializedIdentity;
        this.identity = await createSiwaIdentity(data);

        if (!this.identity.isExpired()) {
          // Restore session key if stored
          const storedKey = await this.storage.get("siwa_session_key");
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
      await this.storage.remove("siwa_identity");
      await this.storage.remove("siwa_session_key");
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
   * Create an anonymous agent for canister calls
   */
  private async createAnonymousAgent(): Promise<HttpAgent> {
    const agent = await HttpAgent.create({
      host: this.host,
    });

    // Fetch root key in non-production environments
    if (
      this.host.includes("localhost") ||
      this.host.includes("127.0.0.1") ||
      this.host.includes(".local")
    ) {
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
   * Prepare a login message for signing
   */
  async prepareLogin(address: string): Promise<PreparedLogin> {
    try {
      const actor = await this.createProviderActor();
      const response: Result_5 = await actor.siwa_prepare_login(address);

      if ("Err" in response) {
        throw new SiwaError(
          SiwaErrorCode.CanisterError,
          response.Err,
          response
        );
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
    try {
      // Generate session key if not provided
      this.sessionKey = sessionKey ?? generateSessionKey();
      const sessionKeyDer = this.sessionKey.getPublicKey().toDer();
      // Convert to number[] for Candid encoding (blob = vec nat8)
      const sessionKeyBytes = Array.from(new Uint8Array(sessionKeyDer));

      // Call siwa_login
      const actor = await this.createProviderActor();
      const loginResponse: Result_4 = await actor.siwa_login(
        signature,
        address,
        sessionKeyBytes
      );

      if ("Err" in loginResponse) {
        throw new SiwaError(
          SiwaErrorCode.CanisterError,
          loginResponse.Err,
          loginResponse
        );
      }

      // LoginResponse contains user_principal and expiration
      const {user_principal: principal, expiration: loginExpiration} =
        loginResponse.Ok;

      // Use the expiration from login response, or default to 30 minutes
      const expirationNs =
        loginExpiration ??
        BigInt(Date.now() + 30 * 60 * 1000) * BigInt(1_000_000);

      const delegationResponse: Result_3 = await actor.siwa_get_delegation(
        address,
        sessionKeyBytes,
        expirationNs
      );

      if ("Err" in delegationResponse) {
        throw new SiwaError(
          SiwaErrorCode.CanisterError,
          delegationResponse.Err,
          delegationResponse
        );
      }

      // Create delegation chain from canister response
      const {delegation: candidDelegation, signature: delegationSignature} =
        delegationResponse.Ok;

      // Convert signature to proper Uint8Array if needed
      const signatureBytes =
        delegationSignature instanceof Uint8Array
          ? delegationSignature
          : new Uint8Array(delegationSignature);

      // Build delegation chain from the canister's delegation
      const delegationChain = this.buildDelegationChain(
        candidDelegation,
        signatureBytes
      );

      // Calculate expiration in milliseconds
      const expirationMs = Number(expirationNs / BigInt(1_000_000));

      // Create serialized identity data
      const serializedIdentity: SerializedIdentity = {
        baseKey: JSON.stringify(this.sessionKey.toJSON()),
        delegationChain: JSON.stringify(delegationChain.toJSON()),
        expiration: expirationMs,
        address,
      };

      // Create identity
      this.identity = await createSiwaIdentity(serializedIdentity);

      // Store identity and session key
      await this.storage.set(
        "siwa_identity",
        JSON.stringify(serializedIdentity)
      );
      await this.storage.set(
        "siwa_session_key",
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
   * Build delegation chain from canister response
   *
   * @param candidDelegation - Delegation record from canister
   * @param signatureBytes - Signature from canister
   */
  private buildDelegationChain(
    candidDelegation: CandidDelegation,
    signatureBytes: Uint8Array
  ): DelegationChain {
    if (!this.sessionKey) {
      throw new SiwaError(
        SiwaErrorCode.NotAuthenticated,
        "No session key available"
      );
    }

    // Convert pubkey from the canister response to ArrayBuffer
    const pubkeyBytes =
      candidDelegation.pubkey instanceof Uint8Array
        ? candidDelegation.pubkey
        : new Uint8Array(candidDelegation.pubkey);

    // Create a proper ArrayBuffer copy from the Uint8Array
    const pubkeyBuffer = new ArrayBuffer(pubkeyBytes.length);
    new Uint8Array(pubkeyBuffer).set(pubkeyBytes);

    // Create Delegation instance from canister response
    // Note: targets are optional in the candid type
    const targets = candidDelegation.targets[0]; // opt vec principal -> [] | [Principal[]]
    const delegation = new Delegation(
      pubkeyBuffer,
      candidDelegation.expiration,
      targets
    );

    // Create delegation chain with single delegation signed by canister
    // Convert signature to ArrayBuffer
    const signatureBuffer = new ArrayBuffer(signatureBytes.length);
    new Uint8Array(signatureBuffer).set(signatureBytes);

    return DelegationChain.fromDelegations(
      [
        {
          delegation,
          signature: signatureBuffer as Signature,
        },
      ],
      pubkeyBuffer
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

    try {
      const actor = await this.createProviderActor();

      // Get new delegation
      const expirationNs =
        BigInt(Date.now() + 30 * 60 * 1000) * BigInt(1_000_000);
      const delegationResponse: Result_3 = await actor.siwa_get_delegation(
        address,
        sessionKeyBytes,
        expirationNs
      );

      if ("Err" in delegationResponse) {
        // Session may have expired, need to re-login
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
        signatureBytes
      );

      const expirationMs = Number(expirationNs / BigInt(1_000_000));

      const serializedIdentity: SerializedIdentity = {
        baseKey: JSON.stringify(this.sessionKey.toJSON()),
        delegationChain: JSON.stringify(delegationChain.toJSON()),
        expiration: expirationMs,
        address,
      };

      this.identity = await createSiwaIdentity(serializedIdentity);
      await this.storage.set(
        "siwa_identity",
        JSON.stringify(serializedIdentity)
      );

      // Reset agent
      this.agent = null;

      // Reschedule refresh
      if (this.autoRefresh) {
        this.scheduleRefresh();
      }

      return {
        principal: this.identity.getPrincipal(),
        address,
        expiration: expirationMs,
      };
    } catch (error) {
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
      this.refreshTimer = setTimeout(() => {
        this.refreshDelegation().catch(() => {
          // Refresh failed, will be handled on next auth check
        });
      }, timeUntilRefresh);
    }
  }

  /**
   * Login with a wallet client (convenience method)
   *
   * @param walletClient - A viem wallet client or similar
   */
  async loginWithWallet(walletClient: {
    account: {address: string};
    signMessage: (args: {message: string}) => Promise<string>;
  }): Promise<LoginResult> {
    const address = walletClient.account.address;

    // Prepare login message
    const prepared = await this.prepareLogin(address);

    // Sign message with wallet
    const signature = await walletClient.signMessage({
      message: prepared.message,
    });

    // Complete login
    return this.login(signature, address);
  }

  /**
   * Logout and clear stored identity
   */
  async logout(): Promise<void> {
    // Clear refresh timer
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer);
      this.refreshTimer = null;
    }

    this.identity = null;
    this.agent = null;
    this.sessionKey = null;

    await this.storage.remove("siwa_identity");
    await this.storage.remove("siwa_session_key");
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

      // Fetch root key in non-production environments
      if (
        this.host.includes("localhost") ||
        this.host.includes("127.0.0.1") ||
        this.host.includes(".local")
      ) {
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
