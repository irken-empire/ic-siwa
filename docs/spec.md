# Functional Specification

## IC-SIWA: Sign-In with Avalanche for the Internet Computer

> **Version**: 0.1.0
> **Status**: Draft
> **Last Updated**: 2026-02-03

This document defines the functional specifications for IC-SIWA, enabling Avalanche wallet-based authentication on the Internet Computer platform.

## Overview

IC-SIWA adapts the Sign-In with Ethereum (SIWE) standard ([EIP-4361](https://eips.ethereum.org/EIPS/eip-4361))

for Avalanche C-Chain wallets. Users can authenticate with their Avalanche wallet to obtain an ICP principal and delegation identity.

### Key Properties

- **Cross-chain Authentication**: Bridge Avalanche identities to ICP principals
- **Deterministic Principal Generation**: Same wallet always produces the same principal (per application)
- **Session-based Delegation**: Time-limited sessions with cryptographic delegation
- **Privacy Preserving**: Canister never has access to user's private keys

## Supported Networks

| Network           | Chain ID | Description                     |
| ----------------- | -------- | ------------------------------- |
| Avalanche Mainnet | 43114    | Production deployments          |
| Avalanche Fuji    | 43113    | Testnet/development deployments |

## Authentication Flow

### Three-Step Login Process

```text
┌─────────┐          ┌──────────────────┐          ┌─────────────┐
│  User   │          │  IC-SIWA         │          │  Avalanche  │
│Frontend │          │  Provider        │          │  Wallet     │
└────┬────┘          └────────┬─────────┘          └──────┬──────┘
     │                        │                           │
     │ 1. siwa_prepare_login  │                           │
     │ ─────────────────────> │                           │
     │                        │                           │
     │    SIWA message        │                           │
     │ <───────────────────── │                           │
     │                        │                           │
     │ Sign message           │                           │
     │ ───────────────────────────────────────────────────>
     │                        │                           │
     │             signature  │                           │
     │ <───────────────────────────────────────────────────
     │                        │                           │
     │ 2. siwa_login          │                           │
     │ ─────────────────────> │                           │
     │                        │                           │
     │    delegation info     │                           │
     │ <───────────────────── │                           │
     │                        │                           │
     │ 3. siwa_get_delegation │                           │
     │ ─────────────────────> │                           │
     │                        │                           │
     │    signed delegation   │                           │
     │ <───────────────────── │                           │
     │                        │                           │
```

### Step 1: Prepare Login (`siwa_prepare_login`)

**Input**: Avalanche C-Chain address (string)

**Output**: SIWA message and nonce

**Behavior**:

1. Validate the address format (0x-prefixed, 40 hex characters)
2. Generate a cryptographically secure nonce
3. Construct the SIWA message with:
   - Domain from canister settings
   - URI from canister settings
   - Avalanche address
   - Chain ID (43113 for Fuji, 43114 for Mainnet)
   - Nonce
   - Issued At timestamp
   - Expiration Time (based on settings)
   - Statement (optional, from settings)
4. Store the message temporarily for verification
5. Return the message and nonce to the caller

**SIWA Message Format**:

```text
{domain} wants you to sign in with your Avalanche account:
{address}

{statement}

URI: {uri}
Version: 1
Chain ID: {chain_id}
Nonce: {nonce}
Issued At: {issued_at}
Expiration Time: {expiration_time}
```

### Step 2: Login (`siwa_login`)

**Input**:

- Signature (hex-encoded string)
- Avalanche address (string)
- Session public key (bytes)

**Output**: Canister public key and delegation expiration

**Behavior**:

1. Retrieve the stored SIWA message for the address
2. Verify the message has not expired
3. Recover the signer address from the signature using ECDSA recovery
4. Verify the recovered address matches the provided address
5. Generate a deterministic seed from:
   - Salt (secret)
   - Avalanche address
   - (Optionally) URI if configured
6. Create a delegation for the session key
7. Store the address-to-principal mapping
8. Return the canister public key and delegation expiration

### Step 3: Get Delegation (`siwa_get_delegation`)

**Input**:

- Avalanche address (string)
- Session public key (bytes)
- Expiration timestamp (u64)

**Output**: Signed delegation

**Behavior**:

1. Verify a valid login exists for the address and session key
2. Create the delegation chain
3. Generate certified data for the delegation
4. Return the signed delegation

## Canister Interface

### Core Endpoints

| Endpoint              | Type   | Description                            |
| --------------------- | ------ | -------------------------------------- |
| `siwa_prepare_login`  | update | Generate SIWA message for signing      |
| `siwa_login`          | update | Verify signature and create delegation |
| `siwa_get_delegation` | query  | Retrieve signed delegation             |

### Utility Endpoints

| Endpoint             | Type  | Description                                 |
| -------------------- | ----- | ------------------------------------------- |
| `get_principal`      | query | Get ICP principal for Avalanche address     |
| `get_address`        | query | Get Avalanche address for ICP principal     |
| `get_caller_address` | query | Get Avalanche address for calling principal |

### Initialization

The canister must be initialized with:

```candid
type InitArgs = record {
    domain: text;           // Required: Domain for SIWA messages
    uri: text;              // Required: Full URI including scheme
    salt: text;             // Required: Secret salt for principal derivation
    chain_id: nat64;        // Required: Avalanche chain ID (43113 or 43114)
    session_expiration_time: nat64;  // Required: Session TTL in nanoseconds
    allowed_domains: opt vec text;   // Optional: Whitelist of allowed domains
    allowed_canisters: opt vec principal; // Optional: Canister whitelist
};
```

## Security Requirements

### Domain Whitelisting

When `allowed_domains` is configured:

- Only requests from whitelisted domains are accepted
- Prevents unauthorized third parties from using the canister

### Canister Targeting

When `allowed_canisters` is configured:

- Delegations are only valid for specified canisters
- Prevents cross-canister delegation abuse

### Salt Security

- The salt MUST be kept secret
- The salt MUST be different for each deployment
- The salt determines principal derivation - changing it invalidates all existing identities

### Session Management

- Sessions MUST have a bounded expiration time
- Login messages MUST expire within a reasonable window (default: 5 minutes)
- Sessions MUST be cryptographically bound to the session key

## Cryptographic Specifications

### Address Validation

Avalanche C-Chain addresses are Ethereum-compatible:

- 20 bytes (40 hex characters)
- 0x prefix
- EIP-55 checksum encoding for display

### Signature Verification

- Algorithm: ECDSA on secp256k1
- Hash function: Keccak256
- Message prefix: "\x19Avalanche Signed Message:\n" + length + message
- Recovery: Use recovery ID (v) to recover public key

### Principal Derivation

```text
seed = SHA256(salt || address || [uri])
principal = DER_encode(seed)
```

## Configuration Schema

### YAML Configuration Files

All configuration files follow this schema:

```yaml
# Domain configuration
domains:
  allowed:
    - "example.com"
    - "*.example.com"

# Avalanche network settings
avalanche:
  chain_id: 43113 # or 43114 for mainnet
  rpc_url: "https://api.avax-test.network/ext/bc/C/rpc"

# IC network settings
ic:
  network: "local" # or "ic" for mainnet
  canisters:
    ic_siwa_provider: null # Set after deployment

# Security settings
security:
  allowed_canisters: []
  session_expiration_seconds: 1800 # 30 minutes
  login_expiration_seconds: 300 # 5 minutes

# Client library settings
library:
  timeout_ms: 30000
```

## Error Handling

### Error Types

| Error Code             | Description                         |
| ---------------------- | ----------------------------------- |
| `InvalidAddress`       | Malformed Avalanche address         |
| `InvalidSignature`     | Signature verification failed       |
| `SignatureExpired`     | SIWA message has expired            |
| `SessionExpired`       | Session delegation has expired      |
| `UnauthorizedDomain`   | Request from non-whitelisted domain |
| `UnauthorizedCanister` | Delegation target not in whitelist  |
| `NotAuthenticated`     | No valid session for caller         |

### Error Response Format

All errors are returned as `Err(String)` with a descriptive message.

## Client Library Requirements

### TypeScript Library (`ic_siwa_ts`)

Must provide:

- Type definitions for all canister methods
- Helper functions for message signing
- Wallet integration utilities
- Session management

### Rust Library (`ic_siwa`)

Must provide:

- SIWA message construction and parsing
- Signature verification
- Delegation creation and management
- Settings management

## Compliance Notes

### Deviations from EIP-4361

The following EIP-4361 fields are not implemented:

- `not-before`: Not required for this use case
- `request-id`: Not required for this use case
- `resources`: Not required for this use case

### Avalanche-Specific Adaptations

- Chain ID defaults to 43113 (Fuji) or 43114 (Mainnet)
- Message prefix adapted for Avalanche signing
- Address format identical to Ethereum (EIP-55)

## Project Components

### Canisters

| Canister           | Language   | Description                  |
| ------------------ | ---------- | ---------------------------- |
| `ic_siwa_provider` | Rust       | Main authentication canister |
| `test_canister_rs` | Rust       | Integration test canister    |
| `test_canister_ts` | TypeScript | Astro frontend test canister |

### Libraries

| Library      | Language   | Package      | Description                   |
| ------------ | ---------- | ------------ | ----------------------------- |
| `ic_siwa`    | Rust       | crates.io    | Core SIWA logic for canisters |
| `ic_siwa_ts` | TypeScript | npm: ic-siwa | Client library for frontends  |

### Development Tools

| Tool         | Description                               |
| ------------ | ----------------------------------------- |
| `ic-siwa.sh` | CLI for build, test, deploy, version mgmt |
| `devenv.nix` | Reproducible dev environment with Nix     |
| Dependabot   | Automated dependency updates              |

## Versioning

IC-SIWA uses **unified versioning** where all components share the same version:

- **Source of truth**: `Cargo.toml` workspace.package.version
- **Synced files**: All `package.json` files
- **Tooling**: `convco` for conventional commit-based version bumps

### Version Commands

```bash
ic-siwa version           # Show current version
ic-siwa version --bump    # Bump based on conventional commits
ic-siwa version --check   # Verify all versions in sync
ic-siwa version --sync    # Sync all versions to Cargo.toml
ic-siwa version --major   # Force major bump
ic-siwa version --minor   # Force minor bump
ic-siwa version --patch   # Force patch bump
```

## References

- [EIP-4361: Sign-In with Ethereum](https://eips.ethereum.org/EIPS/eip-4361)
- [IC-SIWE (Original Implementation)](https://github.com/kristoferlund/ic-siwe)
- [Internet Computer Developer Docs](https://internetcomputer.org/docs/)
- [Avalanche C-Chain Documentation](https://docs.avax.network/)
