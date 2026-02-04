# Funding

Instructions for funding the Canisters with cycles.

## Prerequisites

You need ICP tokens in your identity's ledger account to convert to cycles. You can:

1. Buy ICP on an exchange and transfer to your principal
2. Receive ICP from another wallet

To get your principal address for receiving ICP:

```bash
dfx identity use ic-siwa-testnet  # or ic-siwa-mainnet
dfx identity get-principal
```

To get your account ID (for receiving ICP from exchanges):

```bash
dfx ledger account-id
```

## Testnet

```bash
# Switch to testnet identity
dfx identity use ic-siwa-testnet

# Check ICP balance
dfx ledger balance --network testnet

# Check cycles balance
dfx cycles balance --network testnet

# Convert ICP to cycles (e.g., 1 ICP)
# This creates a cycles wallet if you don't have one
dfx cycles convert --amount 1 --network testnet

# Or convert a specific amount of ICP to cycles
dfx cycles convert --icp 0.5 --network testnet

# Create empty canister (pre-allocate canister ID)
dfx canister create ic_siwa_provider --network testnet

# Check the created canister ID
dfx canister id ic_siwa_provider --network testnet
```

## Mainnet

```bash
# Switch to mainnet identity
dfx identity use ic-siwa-mainnet

# Check ICP balance
dfx ledger balance --network mainnet

# Check cycles balance
dfx cycles balance --network mainnet

# Convert ICP to cycles (e.g., 1 ICP)
dfx cycles convert --amount 1 --network mainnet

# Create empty canister (pre-allocate canister ID)
dfx canister create ic_siwa_provider --network mainnet

# Check the created canister ID
dfx canister id ic_siwa_provider --network mainnet
```

## Checking Canister Cycles Balance

After deployment, you can check a canister's cycles balance:

```bash
# Testnet
dfx canister status ic_siwa_provider --network testnet

# Mainnet
dfx canister status ic_siwa_provider --network mainnet
```

## Topping Up Canisters

To add more cycles to an existing canister:

```bash
# Add 1T cycles to canister (testnet)
dfx canister deposit-cycles 1000000000000 ic_siwa_provider --network testnet

# Add 1T cycles to canister (mainnet)
dfx canister deposit-cycles 1000000000000 ic_siwa_provider --network mainnet
```

## Cost Estimates

- Creating a canister: ~100B cycles (~$0.13)
- Deploying ic_siwa_provider: ~1-2T cycles (~$1.30-2.60)
- Recommended initial funding: 5T cycles (~$6.50)

Note: 1T (trillion) cycles ≈ $1.30 USD (as of 2024)
