/**
 * Candid interface for ic_siwa_provider canister
 * Auto-generated from ic_siwa_provider.did
 */

import type {ActorMethod} from "@dfinity/agent";
import type {IDL} from "@dfinity/candid";
import type {Principal} from "@dfinity/principal";

export interface InitArgs {
  domain: string;
  uri: string;
  salt: string;
  chain_id: bigint;
  session_expiration_time: bigint;
  allowed_domains: [] | [Array<string>];
  allowed_canisters: [] | [Array<Principal>];
}

export interface SignedDelegation {
  delegation: Uint8Array | number[];
  signature: Uint8Array | number[];
}

export type GetDelegationResponse = {Ok: SignedDelegation} | {Err: string};

export type LoginResponse = {Ok: Principal} | {Err: string};

export type PrepareLoginResponse = {Ok: string} | {Err: string};

export type AddressResponse = {Ok: string} | {Err: string};

export type PrincipalResponse = {Ok: Principal} | {Err: string};

export interface _SERVICE {
  siwa_prepare_login: ActorMethod<[string], PrepareLoginResponse>;
  siwa_login: ActorMethod<
    [string, string, Uint8Array | number[]],
    LoginResponse
  >;
  siwa_get_delegation: ActorMethod<
    [string, Uint8Array | number[], bigint],
    GetDelegationResponse
  >;
  get_principal: ActorMethod<[string], PrincipalResponse>;
  get_address: ActorMethod<[Principal], AddressResponse>;
  get_caller_address: ActorMethod<[], AddressResponse>;
}

export const idlFactory = ({IDL}: {IDL: IDL}): IDL.ServiceClass => {
  const InitArgs = IDL.Record({
    domain: IDL.Text,
    uri: IDL.Text,
    salt: IDL.Text,
    chain_id: IDL.Nat64,
    session_expiration_time: IDL.Nat64,
    allowed_domains: IDL.Opt(IDL.Vec(IDL.Text)),
    allowed_canisters: IDL.Opt(IDL.Vec(IDL.Principal)),
  });

  const SignedDelegation = IDL.Record({
    delegation: IDL.Vec(IDL.Nat8),
    signature: IDL.Vec(IDL.Nat8),
  });

  const GetDelegationResponse = IDL.Variant({
    Ok: SignedDelegation,
    Err: IDL.Text,
  });

  const LoginResponse = IDL.Variant({
    Ok: IDL.Principal,
    Err: IDL.Text,
  });

  const PrepareLoginResponse = IDL.Variant({
    Ok: IDL.Text,
    Err: IDL.Text,
  });

  const AddressResponse = IDL.Variant({
    Ok: IDL.Text,
    Err: IDL.Text,
  });

  const PrincipalResponse = IDL.Variant({
    Ok: IDL.Principal,
    Err: IDL.Text,
  });

  return IDL.Service({
    siwa_prepare_login: IDL.Func([IDL.Text], [PrepareLoginResponse], []),
    siwa_login: IDL.Func(
      [IDL.Text, IDL.Text, IDL.Vec(IDL.Nat8)],
      [LoginResponse],
      []
    ),
    siwa_get_delegation: IDL.Func(
      [IDL.Text, IDL.Vec(IDL.Nat8), IDL.Nat64],
      [GetDelegationResponse],
      ["query"]
    ),
    get_principal: IDL.Func([IDL.Text], [PrincipalResponse], ["query"]),
    get_address: IDL.Func([IDL.Principal], [AddressResponse], ["query"]),
    get_caller_address: IDL.Func([], [AddressResponse], ["query"]),
  });
};

export const init = ({IDL}: {IDL: IDL}): IDL.Type[] => {
  const InitArgs = IDL.Record({
    domain: IDL.Text,
    uri: IDL.Text,
    salt: IDL.Text,
    chain_id: IDL.Nat64,
    session_expiration_time: IDL.Nat64,
    allowed_domains: IDL.Opt(IDL.Vec(IDL.Text)),
    allowed_canisters: IDL.Opt(IDL.Vec(IDL.Principal)),
  });
  return [InitArgs];
};
