/**
 * Astro components for IC-SIWA
 *
 * Usage:
 * ```astro
 * ---
 * import { LoginButton } from 'ic-siwa/astro';
 * ---
 *
 * <LoginButton canisterId="xxxxx-xxxxx-xxxxx-xxxxx-xxx" />
 * ```
 */

// Note: Astro components are exported via package.json exports map
// This file provides documentation and type information

export interface LoginButtonProps {
  /** Canister ID of the ic_siwa_provider */
  canisterId: string;
  /** IC host URL (default: https://ic0.app) */
  host?: string;
  /** Button label text */
  label?: string;
  /** Button size: xs, sm, md, lg, xl */
  size?: "xs" | "sm" | "md" | "lg" | "xl";
  /** Button color variant */
  variant?: "primary" | "secondary" | "accent" | "neutral";
  /** Button style */
  style?: "solid" | "outline" | "soft" | "ghost";
  /** Additional CSS classes */
  class?: string;
  /** Show wallet address when logged in */
  showAddress?: boolean;
  /** Custom ID for the button element */
  id?: string;
}

/**
 * Custom events emitted by LoginButton:
 *
 * - `siwa:login-start` - Emitted when login process begins
 * - `siwa:login-success` - Emitted on successful login
 *   - detail: { principal: string, address: string }
 * - `siwa:login-error` - Emitted on login failure
 *   - detail: { error: string }
 * - `siwa:logout` - Emitted when user logs out
 *
 * @example
 * ```html
 * <div id="login-container">
 *   <LoginButton canisterId="..." />
 * </div>
 *
 * <script>
 *   document.getElementById('login-container')
 *     .addEventListener('siwa:login-success', (e) => {
 *       console.log('Logged in:', e.detail.principal);
 *     });
 * </script>
 * ```
 */
export type LoginButtonEvents = {
  "siwa:login-start": CustomEvent<void>;
  "siwa:login-success": CustomEvent<{principal: string; address: string}>;
  "siwa:login-error": CustomEvent<{error: string}>;
  "siwa:logout": CustomEvent<void>;
};
