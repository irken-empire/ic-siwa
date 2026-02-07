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

### Four-Step Login Process

```text
┌─────────┐          ┌──────────────────┐          ┌─────────────┐
│  User   │          │  IC-SIWA         │          │  Avalanche  │
│Frontend │          │  Provider        │          │  Wallet     │
└────┬────┘          └────────┬─────────┘          └──────┬──────┘
     │                        │                           │
     │ 1. siwa_prepare_login  │                           │
     │    (update)            │                           │
     │ ─────────────────────> │                           │
     │                        │                           │
     │    SIWA message,       │                           │
     │    nonce, expiration   │                           │
     │ <───────────────────── │                           │
     │                        │                           │
     │ Sign message           │                           │
     │ ───────────────────────────────────────────────────>
     │                        │                           │
     │             signature  │                           │
     │ <───────────────────────────────────────────────────
     │                        │                           │
     │ 2. siwa_login          │                           │
     │    (update)            │                           │
     │ ─────────────────────> │                           │
     │                        │                           │
     │    principal,          │                           │
     │    expiration,         │                           │
     │    canister_pubkey     │                           │
     │ <───────────────────── │                           │
     │                        │                           │
     │ 3. siwa_prepare_       │                           │
     │    delegation (update) │                           │
     │ ─────────────────────> │                           │
     │                        │                           │
     │    success             │                           │
     │ <───────────────────── │                           │
     │                        │                           │
     │ 4. siwa_get_delegation │                           │
     │    (query, certified)  │                           │
     │ ─────────────────────> │                           │
     │                        │                           │
     │    signed delegation   │                           │
     │ <───────────────────── │                           │
     │                        │                           │
```

### Step 1: Prepare Login (`siwa_prepare_login`)

**Input**: Avalanche C-Chain address (string)

**Output**: SIWA message, nonce, and expiration timestamp

**Behavior**:

1. Validate the address format (0x-prefixed, 40 hex characters, EIP-55 checksum)
2. Enforce rate limits (per-address and global login attempt limits)
3. Generate a cryptographically secure nonce (via IC randomness)
4. Construct the SIWA message with:
   - Domain from canister settings (or request, for multi-tenant)
   - URI from canister settings (or request, for multi-tenant)
   - Avalanche address
   - Chain ID (43113 for Fuji, 43114 for Mainnet)
   - Nonce
   - Issued At timestamp
   - Expiration Time (based on settings)
   - Statement
5. Store the message temporarily for verification
6. Return the message, nonce, and expiration to the caller

**Multi-Tenant Variant** (`siwa_prepare_login_with_options`):

Accepts a `PrepareLoginRequest` with optional `domain` and `uri` fields, allowing
different frontends to share a single provider canister. When `allowed_domains` is
configured, the provided domain is validated against the whitelist. If not provided,
the canister's default domain and URI are used.

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

**Output**: User principal, delegation expiration, and canister public key

**Behavior**:

1. Validate the address format
2. Retrieve the stored SIWA message for the address
3. Verify the message has not expired
4. Recover the signer address from the signature using ECDSA recovery
5. Verify the recovered address matches the provided address (case-insensitive)
6. Validate domain against whitelist (if configured)
7. Derive a deterministic ICP principal from the address and salt
8. Generate a seed for the delegation chain
9. Create a DER-encoded canister public key for the delegation
10. Calculate session expiration
11. Remove the login session (prevents replay)
12. Store the authenticated session
13. Store the address-to-principal mapping
14. Return the user principal, expiration, and canister public key

### Step 3: Prepare Delegation (`siwa_prepare_delegation`)

**Input**:

- Avalanche address (string)
- Session public key (bytes)
- Expiration timestamp (u64)

**Output**: Success or error

**Behavior**:

1. Clean up any expired prepared delegations
2. Validate the session (verify address, session key, and expiration match a stored session)
3. Cap the requested expiration to the configured session maximum
4. Compute the delegation hash from the session key, expiration, and optional targets
5. Store the delegation hash in the certified data signature map
6. Cache the prepared delegation metadata for retrieval in Step 4

This step is an **update call** because only update calls can modify the canister's
certified data. The certified data is required for the subsequent query call to
return a verifiable delegation.

### Step 4: Get Delegation (`siwa_get_delegation`)

**Input**:

- Avalanche address (string)
- Session public key (bytes)
- Expiration timestamp (u64)

**Output**: Signed delegation (delegation + IC certificate signature)

**Behavior**:

1. Validate the session (same checks as Step 3)
2. Look up the prepared delegation metadata from Step 3
3. Obtain the IC system certificate for the certified data
4. Create a hash tree witness proving the delegation is in the certified data
5. CBOR-encode the certificate and witness into the delegation signature
6. Return the signed delegation with the session key, expiration, and optional targets

This step is a **query call** that returns certified data. It must be preceded by
`siwa_prepare_delegation` (Step 3) which sets up the certified data that this
query reads.

## Canister Interface

### Core Endpoints

| Endpoint                          | Type   | Description                                     |
| --------------------------------- | ------ | ----------------------------------------------- |
| `siwa_prepare_login`              | update | Generate SIWA message for signing               |
| `siwa_prepare_login_with_options` | update | Generate SIWA message with multi-tenant options |
| `siwa_login`                      | update | Verify signature and create session             |
| `siwa_prepare_delegation`         | update | Prepare certified data for delegation query     |
| `siwa_get_delegation`             | query  | Retrieve signed delegation (certified)          |

### Session Management Endpoints

| Endpoint          | Type   | Description                                     |
| ----------------- | ------ | ----------------------------------------------- |
| `siwa_logout`     | update | Revoke a specific session by address and key    |
| `siwa_revoke_all` | update | Revoke all sessions for an address (controller) |

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
- Message prefix: "\x19Ethereum Signed Message:\n" + length + message (Avalanche C-Chain uses the standard Ethereum personal sign prefix for EVM compatibility)
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
- Uses standard Ethereum personal sign prefix (`\x19Ethereum Signed Message:\n`) since Avalanche C-Chain is EVM-compatible
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
