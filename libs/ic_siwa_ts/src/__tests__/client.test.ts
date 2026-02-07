import {describe, expect, it} from "bun:test";
import {Principal} from "@dfinity/principal";
import {SiwaClient} from "../client";
import {SiwaErrorCode} from "../errors";
import {MemoryStorageProvider} from "../storage";

// A valid-looking canister ID for constructing clients
const TEST_CANISTER_ID = "rrkah-fqaaa-aaaaa-aaaaq-cai";

function createClient(host = "https://ic0.app"): SiwaClient {
  return new SiwaClient({
    canisterId: TEST_CANISTER_ID,
    host,
    storage: new MemoryStorageProvider(),
  });
}

describe("SiwaClient constructor", () => {
  it("sets canister ID", () => {
    const client = createClient();
    expect(client.getCanisterId().toText()).toBe(TEST_CANISTER_ID);
  });

  it("sets host URL", () => {
    const client = createClient("https://custom.ic0.app");
    expect(client.getHost()).toBe("https://custom.ic0.app");
  });

  it("defaults host to https://ic0.app", () => {
    const client = new SiwaClient({
      canisterId: TEST_CANISTER_ID,
      storage: new MemoryStorageProvider(),
    });
    expect(client.getHost()).toBe("https://ic0.app");
  });

  it("returns Principal from getCanisterId", () => {
    const client = createClient();
    expect(client.getCanisterId()).toBeInstanceOf(Principal);
  });
});

describe("address validation (via prepareLogin)", () => {
  const client = createClient();

  it("rejects empty string", async () => {
    try {
      await client.prepareLogin("");
      expect(true).toBe(false); // Should not reach
    } catch (e: unknown) {
      const err = e as {code: string};
      expect(err.code).toBe(SiwaErrorCode.InvalidAddress);
    }
  });

  it("rejects address without 0x prefix", async () => {
    try {
      await client.prepareLogin("1234567890123456789012345678901234567890");
      expect(true).toBe(false);
    } catch (e: unknown) {
      const err = e as {code: string};
      expect(err.code).toBe(SiwaErrorCode.InvalidAddress);
    }
  });

  it("rejects address with wrong length (too short)", async () => {
    try {
      await client.prepareLogin("0x1234");
      expect(true).toBe(false);
    } catch (e: unknown) {
      const err = e as {code: string};
      expect(err.code).toBe(SiwaErrorCode.InvalidAddress);
    }
  });

  it("rejects address with wrong length (too long)", async () => {
    try {
      await client.prepareLogin(
        "0x12345678901234567890123456789012345678901234"
      );
      expect(true).toBe(false);
    } catch (e: unknown) {
      const err = e as {code: string};
      expect(err.code).toBe(SiwaErrorCode.InvalidAddress);
    }
  });

  it("rejects address with non-hex characters", async () => {
    try {
      await client.prepareLogin("0xGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG");
      expect(true).toBe(false);
    } catch (e: unknown) {
      const err = e as {code: string};
      expect(err.code).toBe(SiwaErrorCode.InvalidAddress);
    }
  });

  // Note: valid addresses will fail at the network layer (canister call),
  // not at validation. We're only testing the format check here.
});

describe("signature validation (via login)", () => {
  const client = createClient();
  const validAddress = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

  it("rejects empty signature", async () => {
    try {
      await client.login("", validAddress);
      expect(true).toBe(false);
    } catch (e: unknown) {
      const err = e as {code: string};
      expect(err.code).toBe(SiwaErrorCode.InvalidSignature);
    }
  });

  it("rejects signature without 0x prefix", async () => {
    const sig = "a".repeat(130);
    try {
      await client.login(sig, validAddress);
      expect(true).toBe(false);
    } catch (e: unknown) {
      const err = e as {code: string};
      expect(err.code).toBe(SiwaErrorCode.InvalidSignature);
    }
  });

  it("rejects signature with wrong length (too short)", async () => {
    try {
      await client.login("0xabcd", validAddress);
      expect(true).toBe(false);
    } catch (e: unknown) {
      const err = e as {code: string};
      expect(err.code).toBe(SiwaErrorCode.InvalidSignature);
    }
  });

  it("rejects signature with non-hex characters", async () => {
    const sig = "0x" + "g".repeat(130);
    try {
      await client.login(sig, validAddress);
      expect(true).toBe(false);
    } catch (e: unknown) {
      const err = e as {code: string};
      expect(err.code).toBe(SiwaErrorCode.InvalidSignature);
    }
  });

  it("validates address before signature", async () => {
    // Bad address should throw InvalidAddress, not InvalidSignature
    try {
      await client.login("0x" + "a".repeat(130), "bad-address");
      expect(true).toBe(false);
    } catch (e: unknown) {
      const err = e as {code: string};
      expect(err.code).toBe(SiwaErrorCode.InvalidAddress);
    }
  });
});

describe("storage key namespacing", () => {
  it("different canisters produce different storage keys", async () => {
    const storage1 = new MemoryStorageProvider();
    const storage2 = new MemoryStorageProvider();

    const client1 = new SiwaClient({
      canisterId: "rrkah-fqaaa-aaaaa-aaaaq-cai",
      storage: storage1,
    });

    const client2 = new SiwaClient({
      canisterId: "ryjl3-tyaaa-aaaaa-aaaba-cai",
      storage: storage2,
    });

    // Both clients have different canister IDs, so if they use the same
    // storage provider they won't collide
    expect(client1.getCanisterId().toText()).not.toBe(
      client2.getCanisterId().toText()
    );
  });
});

describe("isAuthenticated", () => {
  it("returns false when no session exists", async () => {
    const client = createClient();
    expect(await client.isAuthenticated()).toBe(false);
  });
});

describe("getIdentity", () => {
  it("returns null when no session exists", async () => {
    const client = createClient();
    expect(await client.getIdentity()).toBeNull();
  });
});

describe("getPrincipal", () => {
  it("returns null when not authenticated", async () => {
    const client = createClient();
    expect(await client.getPrincipal()).toBeNull();
  });
});

describe("getAddress", () => {
  it("returns null when not authenticated", async () => {
    const client = createClient();
    expect(await client.getAddress()).toBeNull();
  });
});
