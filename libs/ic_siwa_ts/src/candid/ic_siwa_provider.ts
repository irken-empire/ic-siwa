// @ts-nocheck
/**
 * Auto-generated Candid bindings for ic_siwa_provider canister
 * Generated from: canisters/ic_siwa_provider/ic_siwa_provider.did
 * DO NOT EDIT MANUALLY - regenerate with: ic-siwa candid
 */

/* eslint-disable @typescript-eslint/no-explicit-any */

import type {Principal} from "@dfinity/principal";
import type {ActorMethod} from "@dfinity/agent";
import {IDL} from "@dfinity/candid";

export interface Delegation {
  pubkey: Uint8Array | number[];
  targets: [] | [Array<Principal>];
  expiration: bigint;
}
export interface InitArgs {
  uri: string;
  domain: string;
  salt: string;
  chain_id: bigint;
  allowed_domains: [] | [Array<string>];
  allowed_canisters: [] | [Array<Principal>];
  session_expiration_time: bigint;
}
export interface LoginResponse {
  user_principal: Principal;
  expiration: bigint;
}
export interface PrepareLoginResponse {
  expiration: bigint;
  message: string;
  nonce: string;
}
export type Result = {Ok: string} | {Err: string};
export type Result_1 = {Ok: Principal} | {Err: string};
export type Result_2 = {Ok: SignedDelegation} | {Err: string};
export type Result_3 = {Ok: LoginResponse} | {Err: string};
export type Result_4 = {Ok: PrepareLoginResponse} | {Err: string};
export interface SignedDelegation {
  signature: Uint8Array | number[];
  delegation: Delegation;
}
export interface _SERVICE {
  /**
   * Get Avalanche address for ICP principal
   */
  get_address: ActorMethod<[Principal], Result>;
  /**
   * Get Avalanche address for caller
   */
  get_caller_address: ActorMethod<[], Result>;
  /**
   * Get ICP principal for Avalanche address
   */
  get_principal: ActorMethod<[string], Result_1>;
  /**
   * Get delegation for authenticated principal
   *
   * # Arguments
   * * `address` - The Avalanche address
   * * `session_key` - The session public key from login
   * * `expiration` - Requested expiration timestamp (may be capped)
   *
   * # Returns
   * * `Ok(SignedDelegation)` - The signed delegation for the session
   * * `Err(String)` - Error if not authenticated or expired
   */
  siwa_get_delegation: ActorMethod<
    [string, Uint8Array | number[], bigint],
    Result_2
  >;
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
  siwa_login: ActorMethod<[string, string, Uint8Array | number[]], Result_3>;
  /**
   * Prepare a SIWA login message for signing
   *
   * # Arguments
   * * `address` - The Avalanche address (0x-prefixed, EIP-55 checksummed)
   *
   * # Returns
   * * `Ok(PrepareLoginResponse)` - The message to sign, nonce, and expiration
   * * `Err(String)` - Error description if address is invalid
   */
  siwa_prepare_login: ActorMethod<[string], Result_4>;
}

export const idlFactory = ({IDL}: {IDL: any}) => {
  const InitArgs = IDL.Record({
    uri: IDL.Text,
    domain: IDL.Text,
    salt: IDL.Text,
    chain_id: IDL.Nat64,
    allowed_domains: IDL.Opt(IDL.Vec(IDL.Text)),
    allowed_canisters: IDL.Opt(IDL.Vec(IDL.Principal)),
    session_expiration_time: IDL.Nat64,
  });
  const Result = IDL.Variant({Ok: IDL.Text, Err: IDL.Text});
  const Result_1 = IDL.Variant({Ok: IDL.Principal, Err: IDL.Text});
  const Delegation = IDL.Record({
    pubkey: IDL.Vec(IDL.Nat8),
    targets: IDL.Opt(IDL.Vec(IDL.Principal)),
    expiration: IDL.Nat64,
  });
  const SignedDelegation = IDL.Record({
    signature: IDL.Vec(IDL.Nat8),
    delegation: Delegation,
  });
  const Result_2 = IDL.Variant({Ok: SignedDelegation, Err: IDL.Text});
  const LoginResponse = IDL.Record({
    user_principal: IDL.Principal,
    expiration: IDL.Nat64,
  });
  const Result_3 = IDL.Variant({Ok: LoginResponse, Err: IDL.Text});
  const PrepareLoginResponse = IDL.Record({
    expiration: IDL.Nat64,
    message: IDL.Text,
    nonce: IDL.Text,
  });
  const Result_4 = IDL.Variant({
    Ok: PrepareLoginResponse,
    Err: IDL.Text,
  });
  return IDL.Service({
    get_address: IDL.Func([IDL.Principal], [Result], ["query"]),
    get_caller_address: IDL.Func([], [Result], ["query"]),
    get_principal: IDL.Func([IDL.Text], [Result_1], ["query"]),
    siwa_get_delegation: IDL.Func(
      [IDL.Text, IDL.Vec(IDL.Nat8), IDL.Nat64],
      [Result_2],
      ["query"]
    ),
    siwa_login: IDL.Func(
      [IDL.Text, IDL.Text, IDL.Vec(IDL.Nat8)],
      [Result_3],
      []
    ),
    siwa_prepare_login: IDL.Func([IDL.Text], [Result_4], []),
  });
};
export const init = ({IDL}: {IDL: any}) => {
  const InitArgs = IDL.Record({
    uri: IDL.Text,
    domain: IDL.Text,
    salt: IDL.Text,
    chain_id: IDL.Nat64,
    allowed_domains: IDL.Opt(IDL.Vec(IDL.Text)),
    allowed_canisters: IDL.Opt(IDL.Vec(IDL.Principal)),
    session_expiration_time: IDL.Nat64,
  });
  return [InitArgs];
};
