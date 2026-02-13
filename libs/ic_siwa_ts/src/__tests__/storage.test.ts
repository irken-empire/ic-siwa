import {describe, expect, it, beforeEach} from "bun:test";
import {MemoryStorageProvider} from "../storage";

describe("MemoryStorageProvider", () => {
  let storage: MemoryStorageProvider;

  beforeEach(() => {
    storage = new MemoryStorageProvider();
  });

  it("returns null for missing key", async () => {
    expect(await storage.get("nonexistent")).toBeNull();
  });

  it("stores and retrieves a value", async () => {
    await storage.set("key", "value");
    expect(await storage.get("key")).toBe("value");
  });

  it("overwrites existing value", async () => {
    await storage.set("key", "first");
    await storage.set("key", "second");
    expect(await storage.get("key")).toBe("second");
  });

  it("removes a value", async () => {
    await storage.set("key", "value");
    await storage.remove("key");
    expect(await storage.get("key")).toBeNull();
  });

  it("remove is safe for missing key", async () => {
    await storage.remove("nonexistent");
    expect(await storage.get("nonexistent")).toBeNull();
  });

  it("clears all values", async () => {
    await storage.set("a", "1");
    await storage.set("b", "2");
    await storage.set("c", "3");
    storage.clear();
    expect(await storage.get("a")).toBeNull();
    expect(await storage.get("b")).toBeNull();
    expect(await storage.get("c")).toBeNull();
  });

  it("isolates keys between instances", async () => {
    const other = new MemoryStorageProvider();
    await storage.set("key", "from-first");
    await other.set("key", "from-second");
    expect(await storage.get("key")).toBe("from-first");
    expect(await other.get("key")).toBe("from-second");
  });

  it("handles empty string values", async () => {
    await storage.set("key", "");
    expect(await storage.get("key")).toBe("");
  });

  it("handles JSON serialized values", async () => {
    const data = {address: "0xABC", expiration: 12345};
    await storage.set("identity", JSON.stringify(data));
    const retrieved = JSON.parse((await storage.get("identity"))!);
    expect(retrieved).toEqual(data);
  });
});
