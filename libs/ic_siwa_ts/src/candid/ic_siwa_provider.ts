/**
 * Auto-generated Candid bindings for ic_siwa_provider canister
 * Generated from: canisters/ic_siwa_provider/ic_siwa_provider.did
 * DO NOT EDIT MANUALLY - regenerate with: ic-siwa candid
 */

/* eslint-disable @typescript-eslint/no-explicit-any */

import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';

export interface DebugInfo {
  'uri' : string,
  'signature_map_count' : bigint,
  'auth_sessions_count' : bigint,
  'domain' : string,
  'delegation_targets_count' : bigint,
  'chain_id' : bigint,
  'allowed_domains' : Array<string>,
  'allowed_domains_count' : bigint,
  'login_sessions_count' : bigint,
  'rate_limit_total' : number,
  'rate_limit_per_address' : number,
  'prepared_delegations_count' : bigint,
  'session_expiration_ns' : bigint,
  'rate_limit_window_seconds' : bigint,
  'delegation_targets' : Array<Principal>,
  'debug_enabled' : boolean,
}
export interface Delegation {
  'pubkey' : Uint8Array | number[],
  'targets' : [] | [Array<Principal>],
  'expiration' : bigint,
}
export interface InitArgs {
  'uri' : string,
  'domain' : string,
  'salt' : string,
  'chain_id' : bigint,
  'allowed_domains' : [] | [Array<string>],
  'allowed_canisters' : [] | [Array<Principal>],
  'rate_limits' : [] | [RateLimitArgs],
  'delegation_targets' : [] | [Array<Principal>],
  'debug' : [] | [boolean],
  'session_expiration_time' : bigint,
  'login_expiration_time' : [] | [bigint],
}
export interface LoginResponse {
  'user_principal' : Principal,
  'user_canister_pubkey' : Uint8Array | number[],
  'expiration' : bigint,
}
export interface PrepareLoginRequest {
  'uri' : [] | [string],
  'domain' : [] | [string],
  'address' : string,
}
export interface PrepareLoginResponse {
  'expiration' : bigint,
  'message' : string,
  'nonce' : string,
}
export interface RateLimitArgs {
  'max_logins_total' : number,
  'window_seconds' : bigint,
  'max_logins_per_address' : number,
}
export type Result = { 'Ok' : DebugInfo } |
  { 'Err' : string };
export type Result_1 = { 'Ok' : string } |
  { 'Err' : string };
export type Result_2 = { 'Ok' : Principal } |
  { 'Err' : string };
export type Result_3 = { 'Ok' : bigint } |
  { 'Err' : string };
export type Result_4 = { 'Ok' : SignedDelegation } |
  { 'Err' : string };
export type Result_5 = { 'Ok' : LoginResponse } |
  { 'Err' : string };
export type Result_6 = { 'Ok' : null } |
  { 'Err' : string };
export type Result_7 = { 'Ok' : PrepareLoginResponse } |
  { 'Err' : string };
export interface SignedDelegation {
  'signature' : Uint8Array | number[],
  'delegation' : Delegation,
}
export interface _SERVICE {
  /**
   * Get debug diagnostics (restricted to canister controllers)
   * 
   * Returns configuration and state information for debugging.
   * Only callable by canister controllers for security.
   * This is an update call so that caller identity is consensus-verified
   * (query calls cannot reliably enforce access control on the IC).
   */
  'debug_info' : ActorMethod<[], Result>,
  /**
   * Get Avalanche address for ICP principal
   */
  'get_address' : ActorMethod<[Principal], Result_1>,
  /**
   * Get Avalanche address for caller
   */
  'get_caller_address' : ActorMethod<[], Result_1>,
  /**
   * Get ICP principal for Avalanche address
   */
  'get_principal' : ActorMethod<[string], Result_2>,
  /**
   * Purge identity mappings from stable memory in bounded batches (controller-only)
   * 
   * Removes up to 1000 address-to-principal and principal-to-address mappings
   * per call to stay within IC instruction limits. Call repeatedly until the
   * return value is 0 to purge all mappings. Mappings will be re-populated on
   * next login for each address.
   * 
   * # Returns
   * * `Ok(count)` - Number of mappings removed in this call (0 = done)
   * * `Err(String)` - Error if not a controller
   */
  'purge_identity_mappings' : ActorMethod<[], Result_3>,
  /**
   * Get delegation for authenticated principal
   * 
   * This is a **query** call that retrieves the certified delegation.
   * You **must** call `siwa_prepare_delegation` first (an update call) and wait
   * for it to complete before calling this query.
   * 
   * # Arguments
   * * `address` - The Avalanche address
   * * `session_key` - The session public key from login
   * * `expiration` - Requested expiration timestamp (used for validation)
   * 
   * # Returns
   * * `Ok(SignedDelegation)` - The signed delegation for the session
   * * `Err(String)` - Error if not authenticated, expired, or delegation not prepared
   */
  'siwa_get_delegation' : ActorMethod<
    [string, Uint8Array | number[], bigint],
    Result_4
  >,
  /**
   * Complete SIWA login with signed message
   * 
   * # Arguments
   * * `signature` - Hex-encoded signature from the user's wallet
   * * `address` - The Avalanche address that signed the message
   * * `session_key` - The session public key to bind to this authentication
   * 
   * # Returns
   * * `Ok(LoginResponse)` - The derived principal and session expiration
   * * `Err(String)` - Error if signature is invalid or session expired
   */
  'siwa_login' : ActorMethod<[string, string, Uint8Array | number[]], Result_5>,
  /**
   * Combined login and prepare_delegation in a single update call
   * 
   * This reduces the login flow from 2 update calls + 1 query to
   * 1 update call + 1 query, saving ~2 seconds of consensus latency.
   * 
   * # Arguments
   * * `signature` - Hex-encoded signature from the user's wallet
   * * `address` - The Avalanche address that signed the message
   * * `session_key` - The session public key to bind to this authentication
   * 
   * # Returns
   * * `Ok(LoginResponse)` - The derived principal and session expiration
   * * `Err(String)` - Error if signature is invalid or session expired
   */
  'siwa_login_and_prepare' : ActorMethod<
    [string, string, Uint8Array | number[]],
    Result_5
  >,
  /**
   * Logout - revoke a specific session
   * 
   * The caller must be the session owner (matching derived principal) or a
   * canister controller. This prevents unauthenticated third parties from
   * revoking other users' sessions.
   * 
   * # Arguments
   * * `address` - The Avalanche address
   * * `session_key` - The session public key from login
   * 
   * # Returns
   * * `Ok(())` - Session revoked
   * * `Err(String)` - Error if session not found or caller unauthorized
   */
  'siwa_logout' : ActorMethod<[string, Uint8Array | number[]], Result_6>,
  /**
   * Prepare a delegation for an authenticated session
   * 
   * This must be called before `siwa_get_delegation` to store the delegation
   * in the signature map for certified responses.
   * 
   * # Arguments
   * * `address` - The Avalanche address
   * * `session_key` - The session public key from login
   * * `expiration` - Requested expiration timestamp (may be capped)
   * 
   * # Returns
   * * `Ok(u64)` - The final capped expiration timestamp in nanoseconds
   * * `Err(String)` - Error if not authenticated or expired
   */
  'siwa_prepare_delegation' : ActorMethod<
    [string, Uint8Array | number[], bigint],
    Result_3
  >,
  /**
   * Prepare a SIWA login message for signing (simple version)
   * 
   * Uses the canister's default domain/uri from init args.
   * For multi-tenant "SIWA as a Service", use `siwa_prepare_login_with_options` instead.
   * 
   * # Arguments
   * * `address` - The Avalanche address (0x-prefixed, EIP-55 checksummed)
   * 
   * # Returns
   * * `Ok(PrepareLoginResponse)` - The message to sign, nonce, and expiration
   * * `Err(String)` - Error description if address is invalid
   */
  'siwa_prepare_login' : ActorMethod<[string], Result_7>,
  /**
   * Prepare a SIWA login message with custom domain/uri (multi-tenant version)
   * 
   * This is the "SIWA as a Service" endpoint where each calling application
   * can specify its own domain/uri for the wallet signing prompt.
   * 
   * The domain must be in the `allowed_domains` whitelist configured at init.
   * 
   * # Arguments
   * * `request` - Login request containing:
   * - `address`: The Avalanche address (0x-prefixed)
   * - `domain`: Optional domain to show in wallet (must be whitelisted)
   * - `uri`: Optional URI to show in wallet
   * 
   * # Returns
   * * `Ok(PrepareLoginResponse)` - The message to sign, nonce, and expiration
   * * `Err(String)` - Error if address invalid or domain not whitelisted
   */
  'siwa_prepare_login_with_options' : ActorMethod<
    [PrepareLoginRequest],
    Result_7
  >,
  /**
   * Revoke all sessions for an address (controller-only)
   * 
   * Emergency endpoint to revoke all active sessions for a given address.
   * Only callable by canister controllers.
   * 
   * # Arguments
   * * `address` - The Avalanche address to revoke sessions for
   * 
   * # Returns
   * * `Ok(count)` - Number of sessions revoked
   * * `Err(String)` - Error if not a controller
   */
  'siwa_revoke_all' : ActorMethod<[string], Result_3>,
}

export const idlFactory = ({ IDL }: { IDL: any }) => {
  const DebugInfo = IDL.Record({
    'uri' : IDL.Text,
    'signature_map_count' : IDL.Nat64,
    'auth_sessions_count' : IDL.Nat64,
    'domain' : IDL.Text,
    'delegation_targets_count' : IDL.Nat64,
    'chain_id' : IDL.Nat64,
    'allowed_domains' : IDL.Vec(IDL.Text),
    'allowed_domains_count' : IDL.Nat64,
    'login_sessions_count' : IDL.Nat64,
    'rate_limit_total' : IDL.Nat32,
    'rate_limit_per_address' : IDL.Nat32,
    'prepared_delegations_count' : IDL.Nat64,
    'session_expiration_ns' : IDL.Nat64,
    'rate_limit_window_seconds' : IDL.Nat64,
    'delegation_targets' : IDL.Vec(IDL.Principal),
    'debug_enabled' : IDL.Bool,
  });
  const Result = IDL.Variant({ 'Ok' : DebugInfo, 'Err' : IDL.Text });
  const Result_1 = IDL.Variant({ 'Ok' : IDL.Text, 'Err' : IDL.Text });
  const Result_2 = IDL.Variant({ 'Ok' : IDL.Principal, 'Err' : IDL.Text });
  const Result_3 = IDL.Variant({ 'Ok' : IDL.Nat64, 'Err' : IDL.Text });
  const Delegation = IDL.Record({
    'pubkey' : IDL.Vec(IDL.Nat8),
    'targets' : IDL.Opt(IDL.Vec(IDL.Principal)),
    'expiration' : IDL.Nat64,
  });
  const SignedDelegation = IDL.Record({
    'signature' : IDL.Vec(IDL.Nat8),
    'delegation' : Delegation,
  });
  const Result_4 = IDL.Variant({ 'Ok' : SignedDelegation, 'Err' : IDL.Text });
  const LoginResponse = IDL.Record({
    'user_principal' : IDL.Principal,
    'user_canister_pubkey' : IDL.Vec(IDL.Nat8),
    'expiration' : IDL.Nat64,
  });
  const Result_5 = IDL.Variant({ 'Ok' : LoginResponse, 'Err' : IDL.Text });
  const Result_6 = IDL.Variant({ 'Ok' : IDL.Null, 'Err' : IDL.Text });
  const PrepareLoginResponse = IDL.Record({
    'expiration' : IDL.Nat64,
    'message' : IDL.Text,
    'nonce' : IDL.Text,
  });
  const Result_7 = IDL.Variant({
    'Ok' : PrepareLoginResponse,
    'Err' : IDL.Text,
  });
  const PrepareLoginRequest = IDL.Record({
    'uri' : IDL.Opt(IDL.Text),
    'domain' : IDL.Opt(IDL.Text),
    'address' : IDL.Text,
  });
  return IDL.Service({
    'debug_info' : IDL.Func([], [Result], []),
    'get_address' : IDL.Func([IDL.Principal], [Result_1], ['query']),
    'get_caller_address' : IDL.Func([], [Result_1], ['query']),
    'get_principal' : IDL.Func([IDL.Text], [Result_2], ['query']),
    'purge_identity_mappings' : IDL.Func([], [Result_3], []),
    'siwa_get_delegation' : IDL.Func(
        [IDL.Text, IDL.Vec(IDL.Nat8), IDL.Nat64],
        [Result_4],
        ['query'],
      ),
    'siwa_login' : IDL.Func(
        [IDL.Text, IDL.Text, IDL.Vec(IDL.Nat8)],
        [Result_5],
        [],
      ),
    'siwa_login_and_prepare' : IDL.Func(
        [IDL.Text, IDL.Text, IDL.Vec(IDL.Nat8)],
        [Result_5],
        [],
      ),
    'siwa_logout' : IDL.Func([IDL.Text, IDL.Vec(IDL.Nat8)], [Result_6], []),
    'siwa_prepare_delegation' : IDL.Func(
        [IDL.Text, IDL.Vec(IDL.Nat8), IDL.Nat64],
        [Result_3],
        [],
      ),
    'siwa_prepare_login' : IDL.Func([IDL.Text], [Result_7], []),
    'siwa_prepare_login_with_options' : IDL.Func(
        [PrepareLoginRequest],
        [Result_7],
        [],
      ),
    'siwa_revoke_all' : IDL.Func([IDL.Text], [Result_3], []),
  });
};
export const init = ({ IDL }: { IDL: any }) => {
  const RateLimitArgs = IDL.Record({
    'max_logins_total' : IDL.Nat32,
    'window_seconds' : IDL.Nat64,
    'max_logins_per_address' : IDL.Nat32,
  });
  const InitArgs = IDL.Record({
    'uri' : IDL.Text,
    'domain' : IDL.Text,
    'salt' : IDL.Text,
    'chain_id' : IDL.Nat64,
    'allowed_domains' : IDL.Opt(IDL.Vec(IDL.Text)),
    'allowed_canisters' : IDL.Opt(IDL.Vec(IDL.Principal)),
    'rate_limits' : IDL.Opt(RateLimitArgs),
    'delegation_targets' : IDL.Opt(IDL.Vec(IDL.Principal)),
    'debug' : IDL.Opt(IDL.Bool),
    'session_expiration_time' : IDL.Nat64,
    'login_expiration_time' : IDL.Opt(IDL.Nat64),
  });
  return [InitArgs];
};
