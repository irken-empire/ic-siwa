# IC-SIWA

Sign in with Avalanche for the Internet Computer. Build cross-chain Avalanche apps on ICP!

## Overview

IC-SIWA enables Avalanche wallet authentication for Internet Computer applications. Users sign in with their Avalanche wallet (C-Chain) and receive an ICP principal for interacting with canisters.

**Key Features:**

- Avalanche C-Chain wallet authentication
- EIP-4361 (SIWE) compatible message format
- Delegated identity for IC canister calls
- Session management with auto-refresh
- Multi-domain whitelist support
- TypeScript library with Astro components

## Quick Start

### Installation

```bash
# TypeScript/Astro projects
bun add ic-siwa

# Or npm
npm install ic-siwa
```

### Basic Usage

```typescript
import {SiwaClient} from "ic-siwa";

const client = new SiwaClient({
  canisterId: "xxxxx-xxxxx-xxxxx-xxxxx-xxx",
});

// Login with any EVM wallet (MetaMask, Core, etc.)
const address = "0x1234...";
const prepared = await client.prepareLogin(address);
const signature = await wallet.signMessage(prepared.message);
const result = await client.login(signature, address);

console.log("ICP Principal:", result.principal.toText());
```

### Astro Component

```astro
---
import LoginButton from 'ic-siwa/astro';
---

<LoginButton
  canisterId="xxxxx-xxxxx-xxxxx-xxxxx-xxx"
  label="Connect Wallet"
  variant="primary"
/>

<script>
  document.addEventListener('siwa:login-success', (e) => {
    console.log('Logged in:', e.detail.principal);
  });
</script>
```

## Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                         Frontend (Astro)                        │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │  LoginButton    │  │   SiwaClient    │  │  Your App Code  │ │
│  │  (Astro Comp)   │──│  (ic-siwa lib)  │──│                 │ │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘ │
└───────────┼────────────────────┼────────────────────┼──────────┘
            │                    │                    │
            │ 1. Sign Message    │ 2. Login           │ 4. Call
            ▼                    ▼                    ▼
┌───────────────────┐  ┌─────────────────────────────────────────┐
│  Avalanche Wallet │  │           Internet Computer             │
│  (MetaMask, Core) │  │  ┌─────────────────┐  ┌──────────────┐ │
│                   │  │  │ ic_siwa_provider│  │ Your Canister│ │
│  C-Chain (43114)  │  │  │                 │──│              │ │
│  Fuji    (43113)  │  │  │ 3. Verify &     │  │              │ │
└───────────────────┘  │  │    Delegate     │  └──────────────┘ │
                       │  └─────────────────┘                   │
                       └─────────────────────────────────────────┘
```

## Authentication Flow

1. **Prepare Login** - Frontend requests a SIWA message from the canister
2. **Sign Message** - User signs the EIP-4361 message with their Avalanche wallet
3. **Verify & Login** - Canister verifies signature and creates delegation identity
4. **Authenticated** - User receives ICP principal for authenticated canister calls

## Project Structure

```text
ic-siwa/
├── canisters/
│   └── ic_siwa_provider/     # Main authentication canister (Rust)
├── libs/
│   ├── ic_siwa/              # Rust library for canister integration
│   └── ic_siwa_ts/           # TypeScript library for frontends
├── config/
│   ├── development.yaml      # Local development config
│   ├── testnet.yaml          # Avalanche Fuji + IC config
│   └── mainnet.yaml          # Production config
└── docs/                     # Documentation and tickets
```

## TypeScript Library

### SiwaClient

```typescript
import {SiwaClient, SiwaError, SiwaErrorCode} from "ic-siwa";

const client = new SiwaClient({
  canisterId: "xxxxx-xxxxx-xxxxx-xxxxx-xxx",
  host: "https://ic0.app", // Optional, default: https://ic0.app
  autoRefresh: true, // Optional, auto-refresh delegation
});

// Check authentication
if (await client.isAuthenticated()) {
  const principal = await client.getPrincipal();
  const address = await client.getAddress();
}

// Make authenticated canister calls
const agent = await client.getAgent();
const actor = await client.createActor(canisterId, idlFactory);

// Logout
await client.logout();
```

### Astro LoginButton

| Prop          | Type                                  | Default                  | Description                  |
| ------------- | ------------------------------------- | ------------------------ | ---------------------------- |
| `canisterId`  | `string`                              | required                 | IC-SIWA Provider canister ID |
| `host`        | `string`                              | `https://ic0.app`        | IC host URL                  |
| `label`       | `string`                              | `Sign in with Avalanche` | Button text                  |
| `size`        | `xs\|sm\|md\|lg\|xl`                  | `md`                     | Button size (DaisyUI)        |
| `variant`     | `primary\|secondary\|accent\|neutral` | `primary`                | Color                        |
| `style`       | `solid\|outline\|soft\|ghost`         | `solid`                  | Button style                 |
| `showAddress` | `boolean`                             | `true`                   | Show address when logged in  |

**Events:**

- `siwa:login-start` - Login process started
- `siwa:login-success` - Success with `{ principal, address }`
- `siwa:login-error` - Error with `{ error }`
- `siwa:logout` - User logged out

### Error Handling

```typescript
try {
  await client.login(signature, address);
} catch (error) {
  if (error instanceof SiwaError) {
    switch (error.code) {
      case SiwaErrorCode.InvalidSignature:
        // Handle invalid signature
        break;
      case SiwaErrorCode.MessageExpired:
        // Handle expired message
        break;
      case SiwaErrorCode.CanisterError:
        // Handle canister error
        break;
    }
  }
}
```

## Rust Library

### In Your Canister

```rust
use ic_siwa::{SiwaMessage, Settings, validate_address, derive_principal};

// Create settings
let settings = Settings::new("example.com", "https://example.com", "your-salt")
    .with_chain_id(43114)  // Avalanche Mainnet
    .with_session_expiration(30 * 60 * 1_000_000_000); // 30 minutes

// Validate Avalanche address
validate_address("0x1234...")?;

// Derive ICP principal from address
let principal = derive_principal("0x1234...", "your-salt")?;
```

### Domain Validation

```rust
use ic_siwa::types::DomainValidator;

let validator = DomainValidator::new(&[
    "example.com".to_string(),
    "*.example.com".to_string(),  // Wildcard support
]);

if validator.is_allowed("app.example.com") {
    // Domain is allowed
}
```

## Canister API

### Candid Interface

```candid
service : (InitArgs) -> {
    // Authentication
    siwa_prepare_login : (address : text) -> (PrepareLoginResponse);
    siwa_login : (signature : text, address : text, session_key : blob) -> (LoginResponse);
    siwa_get_delegation : (address : text, session_key : blob, expiration : nat64) -> (GetDelegationResponse) query;

    // Lookups
    get_principal : (address : text) -> (PrincipalResponse) query;
    get_address : (principal : principal) -> (AddressResponse) query;
    get_caller_address : () -> (AddressResponse) query;
}
```

### Initialization

```candid
type InitArgs = record {
    domain : text;                        // Your domain (e.g., "example.com")
    uri : text;                           // Your URI (e.g., "https://example.com")
    salt : text;                          // Secret salt for principal derivation
    chain_id : nat64;                     // 43114 (mainnet) or 43113 (fuji)
    session_expiration_time : nat64;      // Nanoseconds (e.g., 30 minutes)
    allowed_domains : opt vec text;       // Domain whitelist
    allowed_canisters : opt vec principal; // Canister whitelist
};
```

## Deployment

### Prerequisites

- [devenv](https://devenv.sh/) with Nix
- Rust with `wasm32-unknown-unknown` target
- Bun or npm

### Local Development

```bash
# Enter development shell
devenv shell

# Start local IC replica
dfx start --background

# Deploy canisters
dfx deploy ic_siwa_provider --argument '(record {
  domain = "localhost";
  uri = "http://localhost:3000";
  salt = "development-salt";
  chain_id = 43113;
  session_expiration_time = 1_800_000_000_000;
  allowed_domains = opt vec { "localhost"; "127.0.0.1" };
  allowed_canisters = null;
})'

# Get canister ID
dfx canister id ic_siwa_provider
```

### Production Deployment

```bash
# Deploy to IC mainnet
dfx deploy ic_siwa_provider --network ic --argument '(record {
  domain = "your-domain.com";
  uri = "https://your-domain.com";
  salt = "YOUR_SECRET_SALT";
  chain_id = 43114;
  session_expiration_time = 1_800_000_000_000;
  allowed_domains = opt vec { "your-domain.com"; "*.your-domain.com" };
  allowed_canisters = null;
})'
```

## Configuration

### Environment Variables

Configuration is loaded from YAML files in `config/`:

| Variable        | Description                     | Example               |
| --------------- | ------------------------------- | --------------------- |
| `SIWA_DOMAIN`   | Primary domain                  | `example.com`         |
| `SIWA_URI`      | Application URI                 | `https://example.com` |
| `SIWA_SALT`     | Secret for principal derivation | (from secrets)        |
| `SIWA_CHAIN_ID` | Avalanche chain ID              | `43114`               |

### Secrets Management

Secrets are defined in `secretspec.toml` and loaded via environment:

```toml
[secrets]
SIWA_SALT = { env = "SIWA_SALT" }
```

## Security

- **Domain Whitelist**: Only configured domains can initiate login
- **Canister Whitelist**: Inter-canister calls restricted to whitelist
- **Session Expiration**: Delegations expire after configured time
- **Nonce Protection**: Replay attacks prevented via nonces
- **EIP-55 Checksums**: Address validation includes checksum verification

## Supported Chains

| Chain             | Chain ID | Network |
| ----------------- | -------- | ------- |
| Avalanche C-Chain | 43114    | Mainnet |
| Avalanche Fuji    | 43113    | Testnet |

## Requirements

- Avalanche-compatible wallet (Core, MetaMask, etc.)
- DaisyUI + Tailwind CSS (for Astro component)
- IC replica or mainnet access

## License

MIT
