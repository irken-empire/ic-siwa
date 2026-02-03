# IC-SIWA TypeScript Library

Sign in with Avalanche for the Internet Computer.

## Installation

```bash
bun add ic-siwa
# or
npm install ic-siwa
```

## Quick Start

```typescript
import {SiwaClient} from "ic-siwa";
import {createWalletClient, http} from "viem";
import {avalanche} from "viem/chains";

// Create SIWA client
const siwa = new SiwaClient({
  canisterId: "xxxxx-xxxxx-xxxxx-xxxxx-xxx",
});

// Create wallet client (using viem)
const walletClient = createWalletClient({
  chain: avalanche,
  transport: http(),
});

// Login with wallet
const result = await siwa.loginWithWallet(walletClient);
console.log("Logged in as:", result.principal.toText());

// Check if authenticated
if (await siwa.isAuthenticated()) {
  const principal = await siwa.getPrincipal();
  console.log("Current principal:", principal?.toText());
}

// Make authenticated calls
const actor = await siwa.createActor(canisterId, idlFactory);
const data = await actor.getData();

// Logout
await siwa.logout();
```

## Manual Login Flow

If you need more control over the login process:

```typescript
const siwa = new SiwaClient({
  canisterId: "xxxxx-xxxxx-xxxxx-xxxxx-xxx",
});

// 1. Prepare login message
const address = "0x1234...abcd";
const prepared = await siwa.prepareLogin(address);

// 2. Sign message with your wallet
const signature = await wallet.signMessage(prepared.message);

// 3. Complete login
const result = await siwa.login(signature, address);
```

## Configuration Options

```typescript
const siwa = new SiwaClient({
  // Required: IC-SIWA Provider canister ID
  canisterId: "xxxxx-xxxxx-xxxxx-xxxxx-xxx",

  // Optional: IC host (default: https://ic0.app)
  host: "https://ic0.app",

  // Optional: Custom storage provider (default: localStorage)
  storage: new MemoryStorageProvider(),

  // Optional: Auto-refresh delegation before expiry (default: true)
  autoRefresh: true,
});
```

## Storage Providers

The library includes two storage providers:

- `LocalStorageProvider` (default) - Persists identity in browser localStorage
- `MemoryStorageProvider` - In-memory storage for testing or SSR

You can also implement your own by implementing the `StorageProvider` interface.

## Error Handling

```typescript
import {SiwaError, SiwaErrorCode} from "ic-siwa";

try {
  await siwa.login(signature, address);
} catch (error) {
  if (error instanceof SiwaError) {
    switch (error.code) {
      case SiwaErrorCode.InvalidSignature:
        console.error("Invalid signature");
        break;
      case SiwaErrorCode.MessageExpired:
        console.error("Login message expired");
        break;
      default:
        console.error("Login failed:", error.message);
    }
  }
}
```

## License

MIT
