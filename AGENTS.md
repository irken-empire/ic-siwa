# AI Agent Instructions

This document provides guidelines for AI agents interacting with the IC-SIWA project.

## Project Overview

IC-SIWA (Sign-In with Avalanche for Internet Computer) is a fork of ic-siwe, adapted for Avalanche authentication on the Internet Computer blockchain. The project provides:

- A canister for handling SIWA authentication (`ic_siwa_provider`)
- A Rust library for SIWA functionality (`ic_siwa`)
- A TypeScript library for client integration (`ic_siwa_ts`)
- Test canisters for integration testing (`test_canister_rs`, `test_canister_ts`)

## Repository Structure

```text
ic-siwa/
├── canisters/                 # IC canisters
│   ├── ic_siwa_provider/      # Main SIWA provider canister
│   ├── test_canister_rs/      # Rust test canister
│   └── test_canister_ts/      # Astro/TypeScript test canister
├── libs/                      # Shared libraries
│   ├── ic_siwa/               # Rust SIWA library
│   └── ic_siwa_ts/            # TypeScript client library
├── config/                    # Environment configurations
│   ├── development.yaml       # Local development settings
│   ├── testnet.yaml           # Avalanche Fuji Testnet settings
│   └── mainnet.yaml           # Avalanche Mainnet settings
├── scripts/                   # Development and deployment scripts
├── docs/                      # Documentation
│   ├── spec.md                # Functional specifications
│   └── todo/                  # Ticket/task tracking
├── .github/workflows/         # CI/CD pipelines
├── Cargo.toml                 # Rust workspace configuration
├── dfx.json                   # DFX canister configuration
├── devenv.nix                 # Development environment (Nix)
└── secretspec.toml            # Secret management configuration
```

## Technology Stack

### Backend (Rust Only)

- **Language**: Rust (stable channel)
- **Target**: `wasm32-unknown-unknown` for IC canisters
- **IC SDK**: `ic-cdk`, `ic-cdk-macros`, `ic-stable-structures`
- **Cryptography**: `k256` for ECDSA, `sha3` for Keccak256

### Frontend (TypeScript/Astro Only)

- **Framework**: Astro
- **Language**: TypeScript
- **Styling**: DaisyUI (Tailwind CSS)

### Development Environment

- **Nix/Devenv**: All dependencies managed via `devenv.nix`
- **IC Tools**: DFX, candid-extractor, ic-wasm
- **Secrets**: Managed via secretspec

## Development Workflow

### Environment Setup

```bash
# Enter development shell (loads all tools and environment)
devenv shell

# Or use direnv for automatic loading
direnv allow

# To run one command inside the shell
devenv shell --quiet -- <command>
```

### Building

```bash
# Build all Rust canisters
cargo build --release --target wasm32-unknown-unknown

# Build specific canister
dfx build ic_siwa_provider
```

### Testing

```bash
# Run Rust tests
cargo test

# Run pre-commit hooks
op signin && devenv shell --quiet -- pre-commit run
```

### Deployment

```bash
# Local development (dfx replica)
dfx start --background
dfx deploy

# Local development (juno emulator)
dfx deploy --network juno

# IC Mainnet
dfx deploy --network ic
```

## Configuration

### YAML Configuration Files

All non-secret configuration is stored in YAML files under `config/`:

- **development.yaml**: Local development with dfx or juno
- **testnet.yaml**: Avalanche Fuji Testnet (chain ID: 43113)
- **mainnet.yaml**: Avalanche Mainnet (chain ID: 43114)

### Secrets Management

Secrets are managed via `secretspec.toml`. Never commit actual secret values.

## Code Standards

### Rust Guidelines

- Use `rustfmt` for formatting
- Use `clippy` for linting (with warnings as errors)
- Prefer explicit error handling over panics
- Document public APIs with doc comments
- Use workspace dependencies from `Cargo.toml`

### TypeScript Guidelines

- Use Prettier for formatting
- Follow ESLint rules
- Type all exports
- Document functions with JSDoc comments

### Commit Standards

- Use conventional commits (enforced via pre-commit hooks)
- Format: `type(scope): description`
- Types: feat, fix, docs, style, refactor, test, chore

## Agent-Specific Instructions

### When Making Changes

1. Always read relevant source files before making modifications
2. Follow existing code patterns and conventions
3. Run pre-commit hooks before considering work complete
4. Update documentation when changing public APIs

### When Creating New Files

1. Place files in appropriate directories per the structure above
2. Add new Rust crates to the workspace in `Cargo.toml`
3. Add new canisters to `dfx.json`
4. Follow naming conventions: `snake_case` for Rust, `kebab-case` for files

### When Working on Tickets

1. Read the ticket file completely before starting
2. Check off completed items in the ticket as you work
3. Ensure all changes pass pre-commit hooks
4. Wait for user acceptance before moving tickets to done

### Key Files to Reference

- `docs/spec.md`: Functional specifications (source of truth for behavior)
- `docs/todo/*.md`: Active tickets and tasks
- `Cargo.toml`: Workspace dependencies and members
- `dfx.json`: Canister definitions and network configuration
- `devenv.nix`: Development environment and available scripts
