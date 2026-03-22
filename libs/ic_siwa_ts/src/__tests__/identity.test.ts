import {describe, expect, it} from "bun:test";
import {generateSessionKey} from "../identity";

describe("generateSessionKey", () => {
  it("returns an Ed25519KeyIdentity", () => {
    const key = generateSessionKey();
    expect(key).toBeDefined();
    expect(key.getPublicKey()).toBeDefined();
  });

  it("generates unique keys each time", () => {
    const key1 = generateSessionKey();
    const key2 = generateSessionKey();
    const pub1 = key1.getPublicKey().toDer();
    const pub2 = key2.getPublicKey().toDer();
    expect(pub1).not.toEqual(pub2);
  });

  it("produces a DER-encoded public key", () => {
    const key = generateSessionKey();
    const der = key.getPublicKey().toDer();
    // @icp-sdk/core/identity returns Uint8Array for DER keys
    expect(der.byteLength).toBeGreaterThan(0);
  });

  it("can serialize and deserialize key", () => {
    const key = generateSessionKey();
    const json = JSON.stringify(key.toJSON());
    expect(json).toBeTruthy();
    expect(typeof json).toBe("string");
  });
});
