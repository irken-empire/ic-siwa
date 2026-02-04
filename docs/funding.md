# Funding

Instructions for funding the Canisters with cycles.

## Network Terminology

- **testnet**: Canisters on IC mainnet, configured for Avalanche Fuji testnet signatures
- **mainnet**: Canisters on IC mainnet, configured for Avalanche mainnet signatures

Both deploy to the Internet Computer mainnet (ic0.app) - there is no IC testnet.

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

# Check ICP balance
dfx ledger balance --network ic

# Check cycles balance in cycles ledger
dfx cycles balance --network ic

# Convert ICP to cycles (if you have ICP)
dfx cycles convert --amount 1 --network ic

# Create canister with cycles from cycles ledger
# Use --with-cycles to specify initial cycles (500B = 0.5T)
dfx canister create ic_siwa_provider --network ic --with-cycles 500000000000

# IMPORTANT: Now update canister_ids.json for the "testnet" and remove "ic"
# {
#  "ic_siwa_provider": {
#    "testnet": "ejj2n-kqaaa-aaaad-qjlxq-cai"
#  }
#}

# Check the created canister ID using "testnet"
dfx canister id ic_siwa_provider --network testnet
```

### Test Canisters (Testnet Only)

For testnet, you may also want to deploy the test canisters for integration testing:

```bash
# Create test canisters (still using --network ic for cycles ledger)
dfx canister create test_canister_rs --network ic --with-cycles 500000000000
dfx canister create test_canister_ts --network ic --with-cycles 500000000000

# Update canister_ids.json to add testnet entries:
# {
#   "ic_siwa_provider": {
#     "testnet": "<provider-canister-id>"
#   },
#   "test_canister_rs": {
#     "testnet": "<test-rs-canister-id>"
#   },
#   "test_canister_ts": {
#     "testnet": "<test-ts-canister-id>"
#   }
# }

# Verify canister IDs
dfx canister id test_canister_rs --network testnet
dfx canister id test_canister_ts --network testnet
```

## Mainnet (Avalanche C-Chain)

```bash
# Switch to mainnet identity
dfx identity use ic-siwa-mainnet

# Check ICP balance
dfx ledger balance --network ic

# Check cycles balance in cycles ledger
dfx cycles balance --network ic

# Convert ICP to cycles (if you have ICP)
dfx cycles convert --amount 1 --network ic

# Create canister with cycles from cycles ledger
# NOTE: Must use --network ic for canister creation
dfx canister create ic_siwa_provider --network ic --with-cycles 500000000000

# IMPORTANT: Update canister_ids.json to add "mainnet" entry and remove "ic"
# {
#   "ic_siwa_provider": {
#     "testnet": "<testnet-canister-id>",
#     "mainnet": "<mainnet-canister-id-from-above>"
#   }
# }

# Check the created canister ID using "mainnet"
dfx canister id ic_siwa_provider --network mainnet
```

- **REMINDER:** Once you have the canister IDs, update the YAML configuration file to use the correct canister IDs.

## Checking Canister Cycles Balance

After deployment, you can check a canister's cycles balance:

```bash
# Testnet
dfx canister status ic_siwa_provider --network testnet

# Mainnet
dfx canister status ic_siwa_provider --network mainnet
```

## Topping Up Canisters

To add more cycles to an existing canisters:

- For `testnet`

```bash
dfx identity use ic-siwa-testnet
dfx cycles balance --network testnet
dfx identity get-principal

# Top up each canister with 1T cycles
#dfx canister deposit-cycles 1000000000000 ic_siwa_provider --network testnet
#dfx canister deposit-cycles 1000000000000 test_canister_rs --network testnet
#dfx canister deposit-cycles 1000000000000 test_canister_ts --network testnet
dfx canister deposit-cycles 1000000000000 ejj2n-kqaaa-aaaad-qjlxq-cai --network ic
dfx canister deposit-cycles 500000000000 qyw5d-liaaa-aaaai-avgna-cai --network ic
dfx canister deposit-cycles 500000000000 kelzz-6qaaa-aaaak-qwlga-cai --network ic
```

- For `mainnet`

```bash
dfx identity use ic-siwa-mainnet
dfx cycles balance --network mainnet
dfx identity get-principal

# Top up each canister with 1T cycles
#dfx canister deposit-cycles 1000000000000 ic_siwa_provider --network mainnet
dfx canister deposit-cycles 1000000000000 tpmsm-eiaaa-aaaam-qgfvq-cai --network ic
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
