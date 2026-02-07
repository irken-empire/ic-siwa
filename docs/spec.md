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

### Diagnostic Endpoints

| Endpoint     | Type  | Description                                |
| ------------ | ----- | ------------------------------------------ |
| `debug_info` | query | Get canister diagnostics (controller-only) |

### Utility Endpoints

| Endpoint             | Type  | Description                                 |
| -------------------- | ----- | ------------------------------------------- |
| `get_principal`      | query | Get ICP principal for Avalanche address     |
| `get_address`        | query | Get Avalanche address for ICP principal     |
| `get_caller_address` | query | Get Avalanche address for calling principal |

### Initialization

The canister must be initialized with:

```candid
type RateLimitArgs = record {
    max_logins_per_address: nat32;  // Max prepare_login calls per address per window
    max_logins_total: nat32;        // Max total prepare_login calls per window
    window_seconds: nat64;          // Time window duration in seconds
};

type InitArgs = record {
    domain: text;                            // Required: Domain for SIWA messages
    uri: text;                               // Required: Full URI including scheme
    salt: text;                              // Required: Secret salt for principal derivation
    chain_id: nat64;                         // Required: Avalanche chain ID (43113 or 43114)
    session_expiration_time: nat64;          // Required: Session TTL in nanoseconds
    allowed_domains: opt vec text;           // Optional: Domain whitelist for multi-tenant
    allowed_canisters: opt vec principal;    // Optional: Caller canister whitelist
    delegation_targets: opt vec principal;   // Optional: Canisters delegations are valid for
    rate_limits: opt RateLimitArgs;          // Optional: Rate limiting configuration
    debug: opt bool;                         // Optional: Enable debug endpoints (default: false)
};
```

## Multi-Tenant Mode ("SIWA as a Service")

A single IC-SIWA provider canister can serve multiple frontend applications,
each with its own domain and URI. This "SIWA as a Service" pattern allows
different apps to share authentication infrastructure while maintaining
distinct wallet signing prompts.

### How It Works

```text
┌─────────────┐     ┌─────────────┐     ┌──────────────────┐
│  App A      │     │  App B      │     │  IC-SIWA         │
│  game.ex.co │     │  shop.ex.co │     │  Provider        │
└──────┬──────┘     └──────┬──────┘     └────────┬─────────┘
       │                   │                     │
       │ prepare_login_    │                     │
       │ with_options      │                     │
       │ domain:"game..."  │                     │
       │ ────────────────────────────────────────>
       │                   │                     │
       │    message with   │                     │
       │    "game.ex.co"   │                     │
       │ <────────────────────────────────────────
       │                   │                     │
       │                   │ prepare_login_      │
       │                   │ with_options        │
       │                   │ domain:"shop..."    │
       │                   │ ────────────────────>
       │                   │                     │
       │                   │    message with     │
       │                   │    "shop.ex.co"     │
       │                   │ <────────────────────
       │                   │                     │
```

Each application receives a SIWA message containing **its own domain**, so the
user's wallet displays the correct origin. The provider canister validates all
custom domains against the `allowed_domains` whitelist.

### Endpoint: `siwa_prepare_login_with_options`

This is the multi-tenant variant of `siwa_prepare_login`. It accepts a
`PrepareLoginRequest` record with optional domain and URI overrides:

```candid
type PrepareLoginRequest = record {
    address: text;          // Required: Avalanche address (0x-prefixed)
    domain: opt text;       // Optional: Custom domain for wallet prompt
    uri: opt text;          // Optional: Custom URI for wallet prompt
};
```

**Behavior**:

1. If `domain` is provided and `allowed_domains` is configured, validate the
   domain against the whitelist. Reject if not whitelisted.
2. If `domain` is provided and `allowed_domains` is empty, accept any domain
   (not recommended for production).
3. If `domain` is not provided, use the canister's default domain from `InitArgs`.
4. Same fallback logic applies to `uri`.
5. The rest of the flow (nonce generation, message construction, session storage)
   is identical to `siwa_prepare_login`.

### Domain Whitelist Rules

The `allowed_domains` field in `InitArgs` controls which domains are accepted
for multi-tenant login. It supports:

- **Exact match**: `"example.com"` matches only `example.com`
- **Wildcard match**: `"*.example.com"` matches `example.com`, `app.example.com`,
  `a.b.example.com`, etc.
- **Case-insensitive**: All comparisons are lowercased

Domain validation occurs at two points:

1. During `siwa_prepare_login_with_options` — the requested domain is checked
2. During `siwa_login` — the domain in the stored message is re-validated

This double-check prevents a race condition where `allowed_domains` is updated
between prepare and login.

### Fallback Behavior

| `domain` provided? | `allowed_domains` configured? | Result                                    |
| ------------------ | ----------------------------- | ----------------------------------------- |
| Yes                | Yes                           | Validate against whitelist                |
| Yes                | No (empty)                    | Reject (custom domains require whitelist) |
| No                 | Yes or No                     | Use canister default from `InitArgs`      |

### Configuration

Multi-tenant mode is configured via `InitArgs` at canister initialization:

```candid
InitArgs = record {
    domain: text;                        // Default domain (used when none specified)
    uri: text;                           // Default URI (used when none specified)
    allowed_domains: opt vec text;       // Domain whitelist for multi-tenant
    // ... other fields
};
```

The corresponding YAML configuration section:

```yaml
# SIWA domain settings (for multi-tenant deployments)
siwa:
  domain: "app.example.com" # Default domain
  uri: "https://app.example.com" # Default URI

security:
  allowed_domains: # Domain whitelist (supports wildcards)
    - "example.com"
    - "*.example.com"
```

### Security Considerations

- Always configure `allowed_domains` in production to prevent unauthorized
  third parties from using your provider canister
- Each whitelisted domain should correspond to a known, trusted application
- The user's wallet will display the domain from the SIWA message — ensure
  only legitimate domains are whitelisted to prevent phishing
- Principal derivation is based on the wallet address and salt, **not** the
  domain — the same wallet produces the same principal regardless of which
  whitelisted domain was used for login

## Security Requirements

### Domain Whitelisting

When `allowed_domains` is configured:

- Only requests from whitelisted domains are accepted
- Supports wildcard patterns (e.g., `*.example.com`)
- Validated during `siwa_prepare_login_with_options` and `siwa_login`
- Prevents unauthorized third parties from using the canister

### Caller Whitelisting (`allowed_canisters`)

When `allowed_canisters` is configured:

- Only whitelisted canister principals can call the provider canister
- Controls **who can invoke** the provider's endpoints (inter-canister call whitelist)
- If empty, all authenticated callers are allowed

### Delegation Targeting (`delegation_targets`)

When `delegation_targets` is configured:

- Delegations are restricted to the specified canister principals
- Controls **where delegations are valid** (which canisters accept the delegation)
- Included in the `Delegation` record and delegation hash computation
- If empty, delegations are unrestricted and work for any canister

> **Note**: `allowed_canisters` and `delegation_targets` serve different purposes.
> `allowed_canisters` restricts who can **call** the provider, while
> `delegation_targets` restricts which canisters the resulting **delegations work for**.

### Rate Limiting

Rate limiting protects the `siwa_prepare_login` endpoint from abuse:

- **Per-address limit**: Maximum login attempts per address within a time window
- **Global limit**: Maximum total login attempts across all addresses within a time window
- **Sliding window**: Automatically resets when the window expires
- **Cleanup**: Expired entries are periodically removed to prevent memory growth

Default values (when `rate_limits` is not provided):

| Parameter                | Default | Description                         |
| ------------------------ | ------- | ----------------------------------- |
| `max_logins_per_address` | 10      | Max attempts per address per window |
| `max_logins_total`       | 1000    | Max total attempts per window       |
| `window_seconds`         | 3600    | Window duration (1 hour)            |

### Salt Security

The salt is a critical secret used in both principal derivation and delegation
seed generation. Its confidentiality determines the security of the identity
mapping.

**Core Requirements**:

- The salt MUST be kept secret
- The salt MUST be different for each deployment
- The salt determines principal derivation -- changing it invalidates all existing identities
- The salt cannot be rotated at runtime; changing it requires a canister upgrade

**Visibility Properties**:

The salt is passed as plaintext in the Candid `InitArgs` during canister
installation. On the Internet Computer:

- Init args are **NOT** stored in the canister history (only module hash and mode are recorded)
- Init args are **NOT** queryable via any public IC API (`canister_info`, `canister_status`, `read_state`)
- Init args **ARE** visible to all subnet replica nodes during ingress message processing
- The salt stored in canister stable memory is also readable by node operators

This means the salt has no public API exposure, but is not cryptographically
protected from IC infrastructure operators (subnet node operators and boundary
nodes).

**Best Practices**:

- Use a cryptographically random salt of sufficient length (32+ characters)
- Generate a unique salt per deployment environment (development, testnet, mainnet)
- Store salts in a secrets manager, not in source control
- Accept that IC node operators have theoretical access to the salt (this is an inherent property of the IC's trust model, not specific to SIWA)
- If the salt is compromised, an attacker could predict which principal maps to any address, but cannot impersonate users without their wallet private key

### Session Management

- Sessions MUST have a bounded expiration time
- Login messages MUST expire within a reasonable window (default: 5 minutes)
- Sessions MUST be cryptographically bound to the session key
- Maximum 5 concurrent sessions per address (oldest evicted when exceeded)

### Debug Mode

When `debug` is enabled (`true`):

- The `debug_info` query endpoint becomes available
- Only callable by canister controllers for security
- Exposes diagnostic information including:
  - Configuration: domain, URI, chain ID, session expiration
  - Security settings: allowed domains, delegation targets, rate limits
  - State counts: login sessions, auth sessions, prepared delegations, signature map size
- MUST be set to `false` in production deployments

## Canister Upgrade Behavior

The canister uses **stable memory** for data that must persist across upgrades and
**heap memory** for transient data that can be safely lost.

### Persistent Data (Stable Memory)

The following data is stored in stable memory using `ic-stable-structures` and
survives canister upgrades:

| Data                     | Storage Type     | Description                              |
| ------------------------ | ---------------- | ---------------------------------------- |
| Address-to-principal map | `StableBTreeMap` | Identity mappings (address -> principal) |
| Principal-to-address map | `StableBTreeMap` | Reverse identity mappings                |
| Settings                 | `StableCell`     | Candid-encoded canister configuration    |

### Transient Data (Heap Memory)

The following data is stored in heap memory and is **reset on every upgrade**.
This is acceptable because users simply need to re-authenticate:

| Data                 | Description                                   |
| -------------------- | --------------------------------------------- |
| Login sessions       | Pending signature verifications               |
| Auth sessions        | Active authenticated sessions                 |
| Prepared delegations | Delegations awaiting query retrieval          |
| Signature map        | Certified data for delegation queries         |
| Rate limiter state   | Per-address and global login attempt counters |

### Upgrade Modes

The `post_upgrade` hook supports two modes:

1. **With `InitArgs`**: Settings are updated in stable memory; transient state is
   reset. Identity mappings are preserved.
2. **Without `InitArgs`**: Settings are loaded from stable memory; transient state
   is reset. Identity mappings are preserved. The canister traps if no settings
   exist in stable memory (i.e., it was never initialized).

> **Note**: Changing the `salt` in `InitArgs` during an upgrade will cause all
> future principal derivations to differ from existing identity mappings. The
> existing mappings in stable memory will become stale. This is by design — the
> salt is immutable for a given deployment's identity set.

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

The user's ICP principal is derived deterministically from their Avalanche address
and the canister's salt using Keccak256:

```text
normalized_address = lowercase(address)
hash = Keccak256(normalized_address || salt)
principal = Principal::from_bytes(hash[0..28])
```

The first 28 bytes of the hash are used as the principal data (IC principals are
at most 29 bytes). This ensures the same wallet address always produces the same
ICP principal for a given canister deployment.

### Delegation Seed

The delegation chain root key uses a separate derivation with SHA-256 and
length-prefixed inputs:

```text
normalized_address = lowercase(address)
seed = SHA256(len_prefix(salt) || len_prefix(normalized_address))
```

Where `len_prefix(x)` prepends a single byte containing the length of `x`.
The seed is used to construct the DER-encoded canister public key for the
delegation chain using IC's canister signature scheme (OID 1.3.6.1.4.1.56387.1.2).

> **Note**: Principal derivation and delegation seed generation use different hash
> algorithms (Keccak256 vs SHA-256) and different input formats (concatenation vs
> length-prefixed). This is intentional: the principal derivation is
> Ethereum-ecosystem-aligned while the delegation seed follows IC conventions.

## Configuration Schema

### YAML Configuration Files

Configuration files are stored in `config/` with environment-specific settings.
All configuration files follow this schema:

```yaml
# Avalanche network settings
avalanche:
  chain_id: 43113 # 43113 for Fuji testnet, 43114 for mainnet
  rpc_url: "https://api.avax-test.network/ext/bc/C/rpc"

# IC network settings
ic:
  network: "local" # "local" for dfx replica, "ic" for mainnet
  canisters:
    ic_siwa_provider: null # Set after deployment

# SIWA domain settings (for multi-tenant deployments)
siwa:
  domain: "app.example.com" # Domain for SIWA messages
  uri: "https://app.example.com" # Full URI including scheme

# Security settings
security:
  allowed_domains: # Domain whitelist (supports wildcards)
    - "example.com"
    - "*.example.com"
  allowed_canisters: [] # Caller canister whitelist
  delegation_targets: [] # Canisters delegations are valid for
  rate_limits:
    max_logins_per_address: 5 # Per-address limit per window
    max_logins_total: 100 # Global limit per window
    window_seconds: 3600 # Window duration in seconds
  session_expiration_seconds: 1800 # Session TTL (30 minutes)
  login_expiration_seconds: 300 # Login message TTL (5 minutes)

# Client library settings
library:
  timeout_ms: 30000 # Request timeout in milliseconds

# Debug mode (should be false in production)
debug: false
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
| `RateLimited`          | Too many login attempts             |

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
