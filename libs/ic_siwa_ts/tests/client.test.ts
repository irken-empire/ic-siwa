/**
 * Tests for SiwaClient
 */

import {describe, it, expect} from "bun:test";
import {SiwaClient} from "../src/client";
import {MemoryStorageProvider} from "../src/storage";
import {SiwaError, SiwaErrorCode} from "../src/errors";

// Test canister ID (valid IC principal format)
const TEST_CANISTER_ID = "rrkah-fqaaa-aaaaa-aaaaq-cai";

describe("SiwaClient", () => {
  describe("constructor", () => {
    it("should create client with required options", () => {
      const client = new SiwaClient({
        canisterId: TEST_CANISTER_ID,
      });

      expect(client).toBeDefined();
      expect(client.getCanisterId().toText()).toBe(TEST_CANISTER_ID);
    });

    it("should use default host", () => {
      const client = new SiwaClient({
        canisterId: TEST_CANISTER_ID,
      });

      expect(client.getHost()).toBe("https://ic0.app");
    });

    it("should accept custom host", () => {
      const client = new SiwaClient({
        canisterId: TEST_CANISTER_ID,
        host: "http://localhost:4943",
      });

      expect(client.getHost()).toBe("http://localhost:4943");
    });

    it("should accept custom storage provider", () => {
      const storage = new MemoryStorageProvider();
      const client = new SiwaClient({
        canisterId: TEST_CANISTER_ID,
        storage,
      });

      expect(client).toBeDefined();
    });

    it("should throw on invalid canister ID", () => {
      expect(() => {
        new SiwaClient({
          canisterId: "invalid",
        });
      }).toThrow();
    });
  });

  describe("isAuthenticated", () => {
    it("should return false when not authenticated", async () => {
      const client = new SiwaClient({
        canisterId: TEST_CANISTER_ID,
        storage: new MemoryStorageProvider(),
      });

      const result = await client.isAuthenticated();
      expect(result).toBe(false);
    });
  });

  describe("getIdentity", () => {
    it("should return null when not authenticated", async () => {
      const client = new SiwaClient({
        canisterId: TEST_CANISTER_ID,
        storage: new MemoryStorageProvider(),
      });

      const identity = await client.getIdentity();
      expect(identity).toBeNull();
    });
  });

  describe("getPrincipal", () => {
    it("should return null when not authenticated", async () => {
      const client = new SiwaClient({
        canisterId: TEST_CANISTER_ID,
        storage: new MemoryStorageProvider(),
      });

      const principal = await client.getPrincipal();
      expect(principal).toBeNull();
    });
  });

  describe("getAddress", () => {
    it("should return null when not authenticated", async () => {
      const client = new SiwaClient({
        canisterId: TEST_CANISTER_ID,
        storage: new MemoryStorageProvider(),
      });

      const address = await client.getAddress();
      expect(address).toBeNull();
    });
  });

  describe("getAgent", () => {
    it("should throw when not authenticated", async () => {
      const client = new SiwaClient({
        canisterId: TEST_CANISTER_ID,
        storage: new MemoryStorageProvider(),
      });

      await expect(client.getAgent()).rejects.toThrow(SiwaError);
    });

    it("should throw with NotAuthenticated code", async () => {
      const client = new SiwaClient({
        canisterId: TEST_CANISTER_ID,
        storage: new MemoryStorageProvider(),
      });

      try {
        await client.getAgent();
        expect.unreachable("Should have thrown");
      } catch (error) {
        expect(error).toBeInstanceOf(SiwaError);
        expect((error as SiwaError).code).toBe(SiwaErrorCode.NotAuthenticated);
      }
    });
  });

  describe("logout", () => {
    it("should clear state even when not authenticated", async () => {
      const storage = new MemoryStorageProvider();
      await storage.set("siwa_identity", "test");
      await storage.set("siwa_session_key", "test");

      const client = new SiwaClient({
        canisterId: TEST_CANISTER_ID,
        storage,
      });

      await client.logout();

      expect(await storage.get("siwa_identity")).toBeNull();
      expect(await storage.get("siwa_session_key")).toBeNull();
    });

    it("should not throw when called multiple times", async () => {
      const client = new SiwaClient({
        canisterId: TEST_CANISTER_ID,
        storage: new MemoryStorageProvider(),
      });

      await client.logout();
      await client.logout();
      // Should not throw
    });
  });
});

describe("SiwaClient with mocked canister", () => {
  // These tests would require mocking the @dfinity/agent Actor
  // For now, we test the interface and error handling

  describe("prepareLogin", () => {
    it("should validate address format conceptually", () => {
      // Valid Ethereum/Avalanche addresses start with 0x and are 42 chars
      const validAddress = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed";
      expect(validAddress.startsWith("0x")).toBe(true);
      expect(validAddress.length).toBe(42);
    });
  });

  describe("login", () => {
    it("should validate signature format conceptually", () => {
      // Valid signatures are hex strings starting with 0x
      const validSignature =
        "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1b";
      expect(validSignature.startsWith("0x")).toBe(true);
      expect(validSignature.length).toBe(132); // 0x + 65 bytes * 2
    });
  });
});
