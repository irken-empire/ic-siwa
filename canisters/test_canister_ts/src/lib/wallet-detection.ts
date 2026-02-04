/**
 * IC-SIWA Test Canister - Wallet Detection Module
 *
 * Implements EIP-6963 wallet detection (modern standard) with legacy fallback.
 * Detects all injected browser wallets: MetaMask, Core, Rabby, Brave, Frame, etc.
 */

import type {EIP1193Provider} from "viem";
import type {
  DetectedWallet,
  EIP6963AnnounceProviderEvent,
  EIP6963ProviderDetail,
} from "./types";
import {getWalletIcon} from "./types";
import {eip6963Log, walletLog} from "./logger";

/** Store of detected wallets by ID */
const detectedWallets = new Map<string, DetectedWallet>();

/** Callbacks to notify when wallet list changes */
const walletChangeCallbacks = new Set<(wallets: DetectedWallet[]) => void>();

/** Whether EIP-6963 listener has been initialized */
let eip6963Initialized = false;

/**
 * Handle EIP-6963 announceProvider event
 */
function handleAnnounceProvider(event: EIP6963AnnounceProviderEvent): void {
  const {info, provider} = event.detail;

  eip6963Log.info(`Wallet announced: ${info.name}`, {
    uuid: info.uuid,
    rdns: info.rdns,
    hasIcon: !!info.icon,
  });

  // Use rdns as ID if available, otherwise uuid
  const id = info.rdns || info.uuid;

  // Skip if we already have this wallet
  if (detectedWallets.has(id)) {
    eip6963Log.debug(`Wallet already detected: ${info.name}`);
    return;
  }

  const wallet: DetectedWallet = {
    id,
    name: info.name,
    icon: info.icon || getWalletIcon(info.rdns, info.name),
    rdns: info.rdns,
    provider: provider,
    source: "eip6963",
  };

  detectedWallets.set(id, wallet);
  notifyWalletChange();
}

/**
 * Initialize EIP-6963 listener
 * This listens for wallet announcements from all EIP-6963 compatible wallets
 */
export function initEIP6963(): void {
  if (eip6963Initialized) {
    eip6963Log.debug("EIP-6963 already initialized");
    return;
  }

  if (typeof window === "undefined") {
    eip6963Log.warn("Window not available, skipping EIP-6963 init");
    return;
  }

  eip6963Log.info("Initializing EIP-6963 wallet detection");

  // Listen for wallet announcements
  window.addEventListener(
    "eip6963:announceProvider",
    handleAnnounceProvider as EventListener
  );

  // Request wallets to announce themselves
  window.dispatchEvent(new Event("eip6963:requestProvider"));

  eip6963Initialized = true;
  eip6963Log.info("EIP-6963 initialized, requested provider announcements");
}

/**
 * Detect legacy window.ethereum wallets
 * This handles wallets that don't support EIP-6963
 */
export function detectLegacyWallets(): void {
  if (typeof window === "undefined") return;

  const win = window as Window & {
    ethereum?: EIP1193Provider & {
      providers?: EIP1193Provider[];
      isMetaMask?: boolean;
      isCoinbaseWallet?: boolean;
      isRabby?: boolean;
      isCore?: boolean;
      isAvalanche?: boolean;
      isBraveWallet?: boolean;
      isFrame?: boolean;
      isTrust?: boolean;
    };
    avalanche?: EIP1193Provider;
  };

  walletLog.info("Detecting legacy wallets");

  // Check for multiple providers array (some wallets populate this)
  if (win.ethereum?.providers?.length) {
    walletLog.debug(
      `Found ${win.ethereum.providers.length} providers in array`
    );

    for (const provider of win.ethereum.providers) {
      const typedProvider = provider as typeof win.ethereum;
      const name = detectProviderName(typedProvider);
      const id = `legacy-${name.toLowerCase().replace(/\s+/g, "-")}`;

      if (!detectedWallets.has(id)) {
        const wallet: DetectedWallet = {
          id,
          name,
          icon: getWalletIcon(undefined, name),
          provider: provider,
          source: "legacy",
        };
        detectedWallets.set(id, wallet);
        walletLog.info(`Detected legacy wallet: ${name}`);
      }
    }
  } else if (win.ethereum) {
    // Single provider
    const name = detectProviderName(win.ethereum);
    const id = `legacy-${name.toLowerCase().replace(/\s+/g, "-")}`;

    if (!detectedWallets.has(id) && !hasEIP6963Wallet(name)) {
      const wallet: DetectedWallet = {
        id,
        name,
        icon: getWalletIcon(undefined, name),
        provider: win.ethereum,
        source: "legacy",
      };
      detectedWallets.set(id, wallet);
      walletLog.info(`Detected legacy wallet: ${name}`);
    }
  }

  // Check for Core/Avalanche specific provider
  if (win.avalanche && !hasWalletByName("core")) {
    const wallet: DetectedWallet = {
      id: "legacy-core",
      name: "Core",
      icon: getWalletIcon("com.avax.core"),
      provider: win.avalanche,
      source: "legacy",
    };
    detectedWallets.set("legacy-core", wallet);
    walletLog.info("Detected Core wallet via window.avalanche");
  }

  notifyWalletChange();
}

/**
 * Detect provider name from flags
 */
function detectProviderName(
  provider: EIP1193Provider & {
    isMetaMask?: boolean;
    isCoinbaseWallet?: boolean;
    isRabby?: boolean;
    isCore?: boolean;
    isAvalanche?: boolean;
    isBraveWallet?: boolean;
    isFrame?: boolean;
    isTrust?: boolean;
  }
): string {
  if (provider.isMetaMask) return "MetaMask";
  if (provider.isCoinbaseWallet) return "Coinbase Wallet";
  if (provider.isRabby) return "Rabby";
  if (provider.isCore || provider.isAvalanche) return "Core";
  if (provider.isBraveWallet) return "Brave Wallet";
  if (provider.isFrame) return "Frame";
  if (provider.isTrust) return "Trust Wallet";
  return "Browser Wallet";
}

/**
 * Check if we already have a wallet with this name from EIP-6963
 */
function hasEIP6963Wallet(name: string): boolean {
  const lowerName = name.toLowerCase();
  for (const wallet of detectedWallets.values()) {
    if (
      wallet.source === "eip6963" &&
      wallet.name.toLowerCase().includes(lowerName)
    ) {
      return true;
    }
  }
  return false;
}

/**
 * Check if we have a wallet containing the given name
 */
function hasWalletByName(name: string): boolean {
  const lowerName = name.toLowerCase();
  for (const wallet of detectedWallets.values()) {
    if (wallet.name.toLowerCase().includes(lowerName)) {
      return true;
    }
  }
  return false;
}

/**
 * Notify all callbacks of wallet list change
 */
function notifyWalletChange(): void {
  const wallets = getDetectedWallets();
  for (const callback of walletChangeCallbacks) {
    try {
      callback(wallets);
    } catch (e) {
      walletLog.error("Wallet change callback error", e);
    }
  }
}

/**
 * Get all detected wallets
 */
export function getDetectedWallets(): DetectedWallet[] {
  return Array.from(detectedWallets.values());
}

/**
 * Get a specific wallet by ID
 */
export function getWalletById(id: string): DetectedWallet | undefined {
  return detectedWallets.get(id);
}

/**
 * Subscribe to wallet list changes
 */
export function onWalletsChanged(
  callback: (wallets: DetectedWallet[]) => void
): () => void {
  walletChangeCallbacks.add(callback);
  // Immediately call with current wallets
  callback(getDetectedWallets());

  // Return unsubscribe function
  return () => {
    walletChangeCallbacks.delete(callback);
  };
}

/**
 * Initialize all wallet detection methods
 */
export function initWalletDetection(): void {
  walletLog.info("Initializing wallet detection");

  // Start EIP-6963 detection
  initEIP6963();

  // Also check legacy after a short delay (EIP-6963 wallets announce async)
  setTimeout(() => {
    detectLegacyWallets();
  }, 100);

  // Re-check legacy after longer delay for slow wallets
  setTimeout(() => {
    detectLegacyWallets();
  }, 500);
}

/**
 * Clear all detected wallets (for testing)
 */
export function clearDetectedWallets(): void {
  detectedWallets.clear();
  notifyWalletChange();
}

/**
 * Request accounts from a wallet provider
 */
export async function requestAccounts(
  provider: EIP1193Provider
): Promise<string[]> {
  walletLog.info("Requesting accounts from wallet");
  try {
    const accounts = (await provider.request({
      method: "eth_requestAccounts",
    })) as string[];
    walletLog.info(`Got ${accounts.length} account(s)`, {first: accounts[0]});
    return accounts;
  } catch (error) {
    walletLog.error("Failed to request accounts", error);
    throw error;
  }
}

/**
 * Get current chain ID from provider
 */
export async function getChainId(provider: EIP1193Provider): Promise<number> {
  walletLog.debug("Getting chain ID");
  const chainIdHex = (await provider.request({
    method: "eth_chainId",
  })) as string;
  const chainId = parseInt(chainIdHex, 16);
  walletLog.debug(`Chain ID: ${chainId} (${chainIdHex})`);
  return chainId;
}

/**
 * Switch to a specific chain
 */
export async function switchChain(
  provider: EIP1193Provider,
  chainId: number,
  chainConfig?: {
    chainName: string;
    rpcUrls: string[];
    nativeCurrency: {name: string; symbol: string; decimals: number};
    blockExplorerUrls: string[];
  }
): Promise<void> {
  const chainIdHex = `0x${chainId.toString(16)}`;
  walletLog.info(`Switching to chain ${chainId} (${chainIdHex})`);

  try {
    await provider.request({
      method: "wallet_switchEthereumChain",
      params: [{chainId: chainIdHex}],
    });
    walletLog.info("Chain switch successful");
  } catch (error: unknown) {
    // If chain doesn't exist, try to add it
    if (
      error &&
      typeof error === "object" &&
      "code" in error &&
      (error as {code: number}).code === 4902 &&
      chainConfig
    ) {
      walletLog.info("Chain not found, attempting to add it");
      await provider.request({
        method: "wallet_addEthereumChain",
        params: [
          {
            chainId: chainIdHex,
            chainName: chainConfig.chainName,
            rpcUrls: chainConfig.rpcUrls,
            nativeCurrency: chainConfig.nativeCurrency,
            blockExplorerUrls: chainConfig.blockExplorerUrls,
          },
        ],
      });
      walletLog.info("Chain added successfully");
    } else {
      walletLog.error("Failed to switch chain", error);
      throw error;
    }
  }
}

/**
 * Sign a message using personal_sign
 */
export async function signMessage(
  provider: EIP1193Provider,
  message: string,
  address: string
): Promise<string> {
  walletLog.info("Requesting message signature", {
    address,
    messageLength: message.length,
  });
  try {
    const signature = (await provider.request({
      method: "personal_sign",
      params: [message as `0x${string}`, address as `0x${string}`],
    })) as string;
    walletLog.info("Message signed successfully");
    return signature;
  } catch (error) {
    walletLog.error("Failed to sign message", error);
    throw error;
  }
}

/**
 * Setup event listeners on a provider
 */
export function setupProviderListeners(
  provider: EIP1193Provider,
  callbacks: {
    onAccountsChanged?: (accounts: string[]) => void;
    onChainChanged?: (chainId: number) => void;
    onDisconnect?: (error: {code: number; message: string}) => void;
  }
): () => void {
  const handleAccountsChanged = (accounts: string[]) => {
    walletLog.info("Accounts changed", {
      count: accounts.length,
      first: accounts[0],
    });
    callbacks.onAccountsChanged?.(accounts);
  };

  const handleChainChanged = (chainIdHex: string) => {
    const chainId = parseInt(chainIdHex, 16);
    walletLog.info(`Chain changed to ${chainId}`);
    callbacks.onChainChanged?.(chainId);
  };

  const handleDisconnect = (error: {code: number; message: string}) => {
    walletLog.warn("Wallet disconnected", error);
    callbacks.onDisconnect?.(error);
  };

  // Subscribe to events
  const typedProvider = provider as EIP1193Provider & {
    on?: (event: string, handler: (...args: unknown[]) => void) => void;
    removeListener?: (
      event: string,
      handler: (...args: unknown[]) => void
    ) => void;
  };

  typedProvider.on?.(
    "accountsChanged",
    handleAccountsChanged as (...args: unknown[]) => void
  );
  typedProvider.on?.(
    "chainChanged",
    handleChainChanged as (...args: unknown[]) => void
  );
  typedProvider.on?.(
    "disconnect",
    handleDisconnect as (...args: unknown[]) => void
  );

  // Return cleanup function
  return () => {
    typedProvider.removeListener?.(
      "accountsChanged",
      handleAccountsChanged as (...args: unknown[]) => void
    );
    typedProvider.removeListener?.(
      "chainChanged",
      handleChainChanged as (...args: unknown[]) => void
    );
    typedProvider.removeListener?.(
      "disconnect",
      handleDisconnect as (...args: unknown[]) => void
    );
  };
}
