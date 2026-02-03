/**
 * Tests for identity management
 */

import {describe, it, expect} from "bun:test";
import {generateSessionKey} from "../src/identity";

describe("generateSessionKey", () => {
  it("should generate Ed25519 key identity", () => {
    const key = generateSessionKey();

    expect(key).toBeDefined();
    expect(typeof key.getPublicKey).toBe("function");
    expect(typeof key.sign).toBe("function");
  });

  it("should generate unique keys each time", () => {
    const key1 = generateSessionKey();
    const key2 = generateSessionKey();

    const pubkey1 = new Uint8Array(key1.getPublicKey().toDer());
    const pubkey2 = new Uint8Array(key2.getPublicKey().toDer());

    // Compare arrays
    const areEqual =
      pubkey1.length === pubkey2.length &&
      pubkey1.every((val, idx) => val === pubkey2[idx]);

    expect(areEqual).toBe(false);
  });

  it("should generate keys that can sign data", async () => {
    const key = generateSessionKey();
    const data = new Uint8Array([1, 2, 3, 4, 5]);

    const signature = await key.sign(data.buffer as ArrayBuffer);
    expect(signature).toBeDefined();
    expect(signature.byteLength).toBeGreaterThan(0);
  });

  it("should generate keys with valid DER-encoded public keys", () => {
    const key = generateSessionKey();
    const der = key.getPublicKey().toDer();

    // DER-encoded Ed25519 public key should be 44 bytes
    // (12 bytes header + 32 bytes key)
    expect(der.byteLength).toBe(44);
  });
});
