/**
 * SIWA Client for browser-based authentication
 */

import {Actor, HttpAgent, type Identity} from "@dfinity/agent";
import {DelegationChain, Ed25519KeyIdentity} from "@dfinity/identity";
import {Principal} from "@dfinity/principal";
import {
  idlFactory,
  type SiwaProviderService,
  type GetDelegationResponse,
  type LoginResponse,
  type PrepareLoginResponse,
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
  private async createProviderActor(
    agent?: HttpAgent
  ): Promise<SiwaProviderService> {
    const actorAgent = agent ?? (await this.createAnonymousAgent());
    return Actor.createActor<SiwaProviderService>(idlFactory, {
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
      const response: PrepareLoginResponse =
        await actor.siwa_prepare_login(address);

      if ("Err" in response) {
        throw new SiwaError(
          SiwaErrorCode.CanisterError,
          response.Err,
          response
        );
      }

      const message = response.Ok;

      // Extract nonce from message (format: "Nonce: <nonce>")
      const nonceMatch = message.match(/Nonce: ([a-zA-Z0-9]+)/);
      const nonce = nonceMatch ? nonceMatch[1] : "";

      // Extract expiration from message (format: "Expiration Time: <iso8601>")
      const expirationMatch = message.match(
        /Expiration Time: (\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z)/
      );
      const expirationTime = expirationMatch
        ? new Date(expirationMatch[1]).getTime() * 1_000_000 // Convert to nanoseconds
        : BigInt(Date.now() + 5 * 60 * 1000) * BigInt(1_000_000); // Default: 5 minutes

      return {
        message,
        nonce,
        expiration: BigInt(expirationTime),
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
      const sessionKeyBytes = new Uint8Array(
        this.sessionKey.getPublicKey().toDer()
      );

      // Call siwa_login
      const actor = await this.createProviderActor();
      const loginResponse: LoginResponse = await actor.siwa_login(
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

      const principal = loginResponse.Ok;

      // Get delegation with expiration (30 minutes from now in nanoseconds)
      const expirationNs =
        BigInt(Date.now() + 30 * 60 * 1000) * BigInt(1_000_000);

      const delegationResponse: GetDelegationResponse =
        await actor.siwa_get_delegation(address, sessionKeyBytes, expirationNs);

      if ("Err" in delegationResponse) {
        throw new SiwaError(
          SiwaErrorCode.CanisterError,
          delegationResponse.Err,
          delegationResponse
        );
      }

      // Create delegation chain from canister response
      const {delegation, signature: delegationSignature} =
        delegationResponse.Ok;

      // Convert to proper Uint8Array if needed
      const delegationBytes =
        delegation instanceof Uint8Array
          ? delegation
          : new Uint8Array(delegation);
      const signatureBytes =
        delegationSignature instanceof Uint8Array
          ? delegationSignature
          : new Uint8Array(delegationSignature);

      // Build delegation chain
      const delegationChain = this.buildDelegationChain(
        delegationBytes,
        signatureBytes,
        expirationNs
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
   */
  private buildDelegationChain(
    delegationBytes: Uint8Array,
    signatureBytes: Uint8Array,
    expiration: bigint
  ): DelegationChain {
    if (!this.sessionKey) {
      throw new SiwaError(
        SiwaErrorCode.NotAuthenticated,
        "No session key available"
      );
    }

    // The delegation from the canister contains the public key and expiration
    // We need to construct a proper DelegationChain
    const delegation = {
      pubkey: this.sessionKey.getPublicKey().toDer(),
      expiration,
      targets: undefined,
    };

    // Create delegation chain with single delegation signed by canister
    return DelegationChain.fromDelegations(
      [
        {
          delegation,
          signature: signatureBytes,
        },
      ],
      this.sessionKey.getPublicKey().toDer()
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
    const sessionKeyBytes = new Uint8Array(
      this.sessionKey.getPublicKey().toDer()
    );

    try {
      const actor = await this.createProviderActor();

      // Get new delegation
      const expirationNs =
        BigInt(Date.now() + 30 * 60 * 1000) * BigInt(1_000_000);
      const delegationResponse: GetDelegationResponse =
        await actor.siwa_get_delegation(address, sessionKeyBytes, expirationNs);

      if ("Err" in delegationResponse) {
        // Session may have expired, need to re-login
        await this.logout();
        return null;
      }

      const {delegation, signature: delegationSignature} =
        delegationResponse.Ok;

      const delegationBytes =
        delegation instanceof Uint8Array
          ? delegation
          : new Uint8Array(delegation);
      const signatureBytes =
        delegationSignature instanceof Uint8Array
          ? delegationSignature
          : new Uint8Array(delegationSignature);

      const delegationChain = this.buildDelegationChain(
        delegationBytes,
        signatureBytes,
        expirationNs
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
