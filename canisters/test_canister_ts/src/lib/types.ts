/**
 * IC-SIWA Test Canister - Shared Types
 */

import type {EIP1193Provider} from "viem";

/**
 * EIP-6963 Provider Info
 * https://eips.ethereum.org/EIPS/eip-6963
 */
export interface EIP6963ProviderInfo {
  /** Unique identifier for the wallet (UUID) */
  uuid: string;
  /** Human-readable name of the wallet */
  name: string;
  /** Icon as a data URI (base64 or SVG) */
  icon: string;
  /** Reverse DNS identifier (e.g., "io.metamask") */
  rdns: string;
}

/**
 * EIP-6963 Provider Detail
 */
export interface EIP6963ProviderDetail {
  info: EIP6963ProviderInfo;
  provider: EIP1193Provider;
}

/**
 * EIP-6963 Announce Provider Event
 */
export interface EIP6963AnnounceProviderEvent extends CustomEvent {
  type: "eip6963:announceProvider";
  detail: EIP6963ProviderDetail;
}

/**
 * Detected wallet (unified format for both EIP-6963 and legacy)
 */
export interface DetectedWallet {
  /** Unique identifier */
  id: string;
  /** Display name */
  name: string;
  /** Icon (data URI or inline SVG) */
  icon: string;
  /** Reverse DNS identifier (if available) */
  rdns?: string;
  /** The provider instance */
  provider: EIP1193Provider;
  /** Detection source */
  source: "eip6963" | "legacy" | "walletconnect";
}

/**
 * Wallet connection state
 */
export interface WalletState {
  /** Currently selected wallet */
  selectedWallet: DetectedWallet | null;
  /** Connected address */
  address: string | null;
  /** Current chain ID */
  chainId: number | null;
  /** Whether wallet is unlocked */
  isUnlocked: boolean;
  /** Whether authenticated with SIWA */
  isAuthenticated: boolean;
  /** ICP principal (if authenticated) */
  principal: string | null;
}

/**
 * Chain configuration
 */
export interface ChainConfig {
  chainId: number;
  chainName: string;
  nativeCurrency: {
    name: string;
    symbol: string;
    decimals: number;
  };
  rpcUrls: string[];
  blockExplorerUrls: string[];
}

/**
 * App configuration
 */
export interface AppConfig {
  /** SIWA provider canister ID */
  canisterId: string;
  /** IC host URL */
  host: string;
  /** Expected chain ID */
  expectedChainId: number;
  /** Chain name for display */
  chainName: string;
  /** RPC URL */
  rpcUrl: string;
  /** Block explorer URL */
  blockExplorer: string;
}

/**
 * Wallet icon registry
 * Maps rdns or name patterns to SVG icons
 */
export const WALLET_ICONS: Record<string, string> = {
  // MetaMask
  "io.metamask": `<svg viewBox="0 0 35 33" class="w-6 h-6"><path d="M32.958 1l-13.134 9.718 2.442-5.727z" fill="#E17726"/><path d="M2.663 1l13.017 9.809-2.325-5.818zm24.324 22.533l-3.507 5.36 7.505 2.067 2.155-7.318zm-26.85.109l2.142 7.318 7.493-2.067-3.507-5.36z" fill="#E27625"/><path d="M12.18 14.77l-2.09 3.163 7.453.34-.26-8.01zm10.64 0l-5.19-4.6-.17 8.1 7.44-.34zm-10.13 12.53l4.49-2.18-3.88-3.03zm8.54-2.18l4.49 2.18-.61-5.21z" fill="#E27625"/></svg>`,

  // Coinbase Wallet
  "com.coinbase.wallet": `<svg viewBox="0 0 40 40" class="w-6 h-6"><circle cx="20" cy="20" r="20" fill="#0052FF"/><path d="M20 8c6.627 0 12 5.373 12 12s-5.373 12-12 12S8 26.627 8 20 13.373 8 20 8z" fill="#fff"/><path d="M17 17h6v6h-6z" fill="#0052FF"/></svg>`,

  // Core Wallet (Avalanche)
  "com.avax.core": `<svg viewBox="0 0 40 40" class="w-6 h-6"><rect width="40" height="40" rx="8" fill="#000"/><path d="M27.5 25.5h-4.7l-2.8-5-2.8 5h-4.7l7.5-13 7.5 13z" fill="#E84142"/></svg>`,

  // Rabby
  "io.rabby": `<svg viewBox="0 0 40 40" class="w-6 h-6"><circle cx="20" cy="20" r="20" fill="#8697FF"/><ellipse cx="20" cy="22" rx="10" ry="8" fill="#fff"/><circle cx="16" cy="20" r="2" fill="#8697FF"/><circle cx="24" cy="20" r="2" fill="#8697FF"/></svg>`,

  // Brave Wallet
  "com.brave.wallet": `<svg viewBox="0 0 40 40" class="w-6 h-6"><rect width="40" height="40" rx="8" fill="#FB542B"/><path d="M20 8l8 6v12l-8 6-8-6V14l8-6z" fill="#fff"/></svg>`,

  // Frame
  "sh.frame": `<svg viewBox="0 0 40 40" class="w-6 h-6"><rect width="40" height="40" rx="8" fill="#1A1A1A"/><rect x="10" y="10" width="20" height="20" rx="2" stroke="#fff" stroke-width="2" fill="none"/><circle cx="20" cy="20" r="4" fill="#fff"/></svg>`,

  // Trust Wallet
  "com.trustwallet.app": `<svg viewBox="0 0 40 40" class="w-6 h-6"><circle cx="20" cy="20" r="20" fill="#0500FF"/><path d="M20 8c6 0 10 4 10 10s-4 14-10 14S10 24 10 18s4-10 10-10z" fill="#fff" fill-opacity="0.2"/><path d="M20 10v20M12 16c4 6 12 6 16 0" stroke="#fff" stroke-width="2" fill="none"/></svg>`,

  // Rainbow
  "me.rainbow": `<svg viewBox="0 0 40 40" class="w-6 h-6"><rect width="40" height="40" rx="8" fill="#001E59"/><path d="M8 28c0-11 9-20 20-20v4c-8.8 0-16 7.2-16 16H8z" fill="#FF4000"/><path d="M12 28c0-8.8 7.2-16 16-16v4c-6.6 0-12 5.4-12 12h-4z" fill="#FF9901"/><path d="M16 28c0-6.6 5.4-12 12-12v4a8 8 0 00-8 8h-4z" fill="#00E510"/><path d="M20 28a8 8 0 018-8v4a4 4 0 00-4 4h-4z" fill="#01CCF7"/></svg>`,

  // WalletConnect
  walletconnect: `<svg viewBox="0 0 40 40" class="w-6 h-6"><rect width="40" height="40" rx="8" fill="#3B99FC"/><path d="M12.3 16c4.3-4.2 11.1-4.2 15.4 0l.5.5-2 2-.3-.3c-3-2.9-7.8-2.9-10.8 0l-.3.3-2-2 .5-.5zm19 3.5l1.8 1.8-8.2 8-5.8-5.7 2-2 3.8 3.7 6.4-5.8zm-22.6 0l6.4 5.8 3.8-3.7 2 2-5.8 5.7-8.2-8 1.8-1.8z" fill="#fff"/></svg>`,

  // Default fallback
  default: `<svg viewBox="0 0 40 40" class="w-6 h-6"><rect width="40" height="40" rx="8" fill="#6B7280"/><path d="M20 12v16M12 20h16" stroke="#fff" stroke-width="2.5" stroke-linecap="round"/></svg>`,
};

/**
 * Get icon for a wallet by rdns or name
 */
export function getWalletIcon(rdns?: string, name?: string): string {
  // Try rdns first
  if (rdns && WALLET_ICONS[rdns]) {
    return WALLET_ICONS[rdns];
  }

  // Try common name patterns
  const lowerName = (name || "").toLowerCase();
  if (lowerName.includes("metamask")) return WALLET_ICONS["io.metamask"];
  if (lowerName.includes("coinbase"))
    return WALLET_ICONS["com.coinbase.wallet"];
  if (lowerName.includes("core") || lowerName.includes("avalanche"))
    return WALLET_ICONS["com.avax.core"];
  if (lowerName.includes("rabby")) return WALLET_ICONS["io.rabby"];
  if (lowerName.includes("brave")) return WALLET_ICONS["com.brave.wallet"];
  if (lowerName.includes("frame")) return WALLET_ICONS["sh.frame"];
  if (lowerName.includes("trust")) return WALLET_ICONS["com.trustwallet.app"];
  if (lowerName.includes("rainbow")) return WALLET_ICONS["me.rainbow"];

  return WALLET_ICONS.default;
}

/**
 * Ethereum RPC error codes
 */
export const RPC_ERRORS = {
  USER_REJECTED: 4001,
  UNAUTHORIZED: 4100,
  UNSUPPORTED_METHOD: 4200,
  DISCONNECTED: 4900,
  CHAIN_DISCONNECTED: 4901,
  PENDING_REQUEST: -32002,
  CHAIN_NOT_ADDED: 4902,
} as const;

/**
 * Type guard for RPC errors
 */
export function isRpcError(
  error: unknown
): error is {code: number; message: string} {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof (error as {code: unknown}).code === "number"
  );
}
