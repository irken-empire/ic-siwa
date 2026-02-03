# IC-SIWA

Sign in with Avalanche for the Internet Computer. Build cross-chain Avalanche apps on ICP!

## Overview

IC-SIWA enables Avalanche wallet authentication for Internet Computer applications. Users can sign in with their Avalanche wallet (C-Chain) and receive an ICP principal for interacting with canisters.

## Project Structure

```text
ic-siwa/
├── canisters/
│   ├── ic_siwa_provider/    # Main SIWA authentication canister
│   ├── test_canister_rs/    # Rust integration test canister
│   └── test_canister_ts/    # Astro/TypeScript test canister
├── libs/
│   ├── ic_siwa/             # Rust library for canister integration
│   └── ic_siwa_ts/          # TypeScript library for frontend integration
├── config/
│   ├── development.yaml     # Local development config
│   ├── testnet.yaml         # Avalanche Fuji Testnet config
│   └── mainnet.yaml         # Avalanche Mainnet config
├── .github/workflows/       # CI/CD pipelines
└── docs/todo/               # Project backlog and tickets
```

## Authentication Flow

1. **Prepare Login**: Frontend requests a SIWA message from the canister
2. **Sign Message**: User signs the message with their Avalanche wallet
3. **Verify & Login**: Canister verifies signature and creates delegation
4. **Authenticated**: User receives ICP principal for canister calls

## Deployments

| Environment | Avalanche Chain | IC Network | Status |
| ----------- | --------------- | ---------- | ------ |
| Development | Fuji Testnet    | Local      | WIP    |
| Testnet     | Fuji Testnet    | IC Mainnet | WIP    |
| Mainnet     | C-Chain Mainnet | IC Mainnet | WIP    |

## Quick Start

### Prerequisites

- [devenv](https://devenv.sh/) with Nix
- Rust with wasm32-unknown-unknown target
- Bun (for TypeScript)

### Development

```bash
# Enter development shell
devenv shell

# Start local IC replica
dfx start --background

# Build and deploy canisters
dfx deploy

# Run tests
cargo test
```

### Using the TypeScript Library

```typescript
import {SiwaClient} from "ic-siwa";

const client = new SiwaClient({
  canisterId: "xxxxx-xxxxx-xxxxx-xxxxx-xxx",
});

// Login with wallet
const result = await client.loginWithWallet(walletClient);
console.log("Principal:", result.principal.toText());
```

### Using the Rust Library

```rust
use ic_siwa::{SiwaMessage, Settings};

// In your canister
let settings = Settings::new("example.com", "https://example.com", "salt");
let message = SiwaMessage::new(&settings, "0x1234...", &nonce);
```

## Configuration

Configuration is managed via YAML files in `config/`:

- `development.yaml` - Local development settings
- `testnet.yaml` - Avalanche Fuji + IC Mainnet
- `mainnet.yaml` - Avalanche Mainnet + IC Mainnet

Secrets are managed via `secretspec.toml` and loaded via environment variables.

## Supported Domains

- irkenempire.tech
- mahdtech.com
- saltlabs.cloud
- saltlabs.tech
- tresr.community

Additional domains can be added via configuration.

## Security

- Domain whitelist validation
- Canister ID whitelist for inter-canister calls
- Session expiration
- Nonce-based replay protection

## Documentation

See `docs/todo/` for the complete project backlog and implementation tickets.

## License

MIT
