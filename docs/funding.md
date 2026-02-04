# Funding

Instructions for funding the Canisters with cycles.

## Network Terminology

- **testnet**: Canisters on IC mainnet, configured for Avalanche Fuji testnet signatures
- **mainnet**: Canisters on IC mainnet, configured for Avalanche mainnet signatures

Both deploy to the Internet Computer mainnet (ic0.app) - there is no IC testnet.

## Canister ID Files

Canister IDs are stored in environment-specific files:

- `canister_ids.testnet.json` - Testnet canister IDs
- `canister_ids.mainnet.json` - Mainnet canister IDs

To work with a specific environment locally, copy the appropriate file:

```bash
# For testnet work
cp canister_ids.testnet.json canister_ids.json

# For mainnet work
cp canister_ids.mainnet.json canister_ids.json
```

All `dfx` commands then use `--network ic`.

> **Note:** `canister_ids.json` is gitignored. Always edit the environment-specific files and copy them.

## Wallet Terminology

- This is **important:**
  - When sending **ICP**, use the **Account ID**.
  - When sending **cycles**, use the **Principal ID**.

## Prerequisites

You need cycles in your cycles ledger to create and deploy canisters. You can:

1. Buy ICP on an exchange and convert to cycles
2. Use the [Cycles Faucet](https://anv4y-qiaaa-aaaal-qaqxq-cai.ic0.app) for initial funding

To get your principal address:

```bash
dfx identity use ic-siwa-testnet  # or ic-siwa-mainnet
dfx identity get-principal
```

To get your account ID (for receiving ICP from exchanges):

```bash
dfx ledger account-id --network ic
```

## Testnet (Avalanche Fuji)

```bash
# Switch to testnet identity
dfx identity use ic-siwa-testnet

# Copy testnet canister IDs
cp canister_ids.testnet.json canister_ids.json

# Check ICP balance
dfx ledger balance --network ic

# Check cycles balance in cycles ledger
dfx cycles balance --network ic

# Convert ICP to cycles (if you have ICP)
dfx cycles convert --amount 1 --network ic

# Create canister with cycles from cycles ledger
# Use --with-cycles to specify initial cycles (500B = 0.5T)
dfx canister create ic_siwa_provider --network ic --with-cycles 500000000000

# IMPORTANT: Update canister_ids.testnet.json with the new canister ID
# {
#   "ic_siwa_provider": {
#     "ic": "<new-canister-id>"
#   }
# }

# Copy updated file and verify
cp canister_ids.testnet.json canister_ids.json
dfx canister id ic_siwa_provider --network ic
```

### Test Canisters (Testnet Only)

For testnet, you may also want to deploy the test canisters for integration testing:

```bash
# Create test canisters
dfx canister create test_canister_rs --network ic --with-cycles 500000000000
dfx canister create test_canister_ts --network ic --with-cycles 500000000000

# Update canister_ids.testnet.json with all canister IDs:
# {
#   "ic_siwa_provider": {
#     "ic": "<provider-canister-id>"
#   },
#   "test_canister_rs": {
#     "ic": "<test-rs-canister-id>"
#   },
#   "test_canister_ts": {
#     "ic": "<test-ts-canister-id>"
#   }
# }

# Copy and verify
cp canister_ids.testnet.json canister_ids.json
dfx canister id test_canister_rs --network ic
dfx canister id test_canister_ts --network ic
```

## Mainnet (Avalanche C-Chain)

```bash
# Switch to mainnet identity
dfx identity use ic-siwa-mainnet

# Copy mainnet canister IDs
cp canister_ids.mainnet.json canister_ids.json

# Check ICP balance
dfx ledger balance --network ic

# Check cycles balance in cycles ledger
dfx cycles balance --network ic

# Convert ICP to cycles (if you have ICP)
dfx cycles convert --amount 1 --network ic

# Create canister with cycles from cycles ledger
dfx canister create ic_siwa_provider --network ic --with-cycles 500000000000

# IMPORTANT: Update canister_ids.mainnet.json with the new canister ID
# {
#   "ic_siwa_provider": {
#     "ic": "<new-canister-id>"
#   }
# }

# Copy updated file and verify
cp canister_ids.mainnet.json canister_ids.json
dfx canister id ic_siwa_provider --network ic
```

> **REMINDER:** Once you have the canister IDs, update the YAML configuration file to use the correct canister IDs.

## Checking Canister Cycles Balance

After deployment, you can check a canister's cycles balance:

```bash
# For testnet
dfx identity use ic-siwa-testnet
cp canister_ids.testnet.json canister_ids.json
dfx canister status ic_siwa_provider --network ic

# For mainnet
dfx identity use ic-siwa-mainnet
cp canister_ids.mainnet.json canister_ids.json
dfx canister status ic_siwa_provider --network ic
```

## Topping Up Canisters

To add more cycles to existing canisters:

### Testnet

```bash
dfx identity use ic-siwa-testnet
cp canister_ids.testnet.json canister_ids.json

dfx cycles balance --network ic
dfx identity get-principal

# Top up using canister name (requires canister_ids.json)
dfx canister deposit-cycles 1000000000000 ic_siwa_provider --network ic
dfx canister deposit-cycles 500000000000 test_canister_rs --network ic
dfx canister deposit-cycles 500000000000 test_canister_ts --network ic

# Or top up using canister ID directly
# dfx canister deposit-cycles 1000000000000 ejj2n-kqaaa-aaaad-qjlxq-cai --network ic
```

### Mainnet

```bash
dfx identity use ic-siwa-mainnet
cp canister_ids.mainnet.json canister_ids.json

dfx cycles balance --network ic
dfx identity get-principal

# Top up using canister name
dfx canister deposit-cycles 1000000000000 ic_siwa_provider --network ic

# Or top up using canister ID directly
# dfx canister deposit-cycles 1000000000000 tpmsm-eiaaa-aaaam-qgfvq-cai --network ic
```

## Cost Estimates

- Creating a canister: ~500B cycles (~$0.65)
- Deploying ic_siwa_provider: ~1-2T cycles (~$1.30-2.60)
- Recommended initial funding: 2T cycles (~$2.60) per canister

Note: 1T (trillion) cycles ≈ $1.30 USD (as of 2025)

## Cycles Usage by Method

### ic_siwa_provider

| Method                | Type   | Costs Cycles?                                              |
| --------------------- | ------ | ---------------------------------------------------------- |
| `siwa_prepare_login`  | Update | Yes - creates and stores a SIWA message                    |
| `siwa_login`          | Update | Yes - verifies signature, creates delegation, stores state |
| `siwa_get_delegation` | Query  | No - just reads stored delegation                          |
| `get_address`         | Query  | No - reads principal to address mapping                    |
| `get_principal`       | Query  | No - reads address to principal mapping                    |
| `get_caller_address`  | Query  | No - derives address from caller                           |

### What costs more cycles?

- **Compute** - signature verification in `siwa_login` is CPU-intensive
- **Storage** - storing delegations, rate limit tracking
- **Memory** - each stored session uses heap memory

### Rough estimates

- Each login (prepare + login): ~1-10B cycles depending on message size
- With 1 TC (trillion cycles), you could handle roughly **100,000 - 1,000,000 logins**

Once you top up canisters with 1-2 TC each, they should handle many logins before needing a refill.
