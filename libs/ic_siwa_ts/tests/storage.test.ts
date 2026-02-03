/**
 * Tests for storage providers
 */

import {describe, it, expect, beforeEach} from "bun:test";
import {MemoryStorageProvider, LocalStorageProvider} from "../src/storage";

describe("MemoryStorageProvider", () => {
  let storage: MemoryStorageProvider;

  beforeEach(() => {
    storage = new MemoryStorageProvider();
  });

  it("should store and retrieve values", async () => {
    await storage.set("key1", "value1");
    const result = await storage.get("key1");
    expect(result).toBe("value1");
  });

  it("should return null for non-existent keys", async () => {
    const result = await storage.get("nonexistent");
    expect(result).toBeNull();
  });

  it("should remove values", async () => {
    await storage.set("key1", "value1");
    await storage.remove("key1");
    const result = await storage.get("key1");
    expect(result).toBeNull();
  });

  it("should overwrite existing values", async () => {
    await storage.set("key1", "value1");
    await storage.set("key1", "value2");
    const result = await storage.get("key1");
    expect(result).toBe("value2");
  });

  it("should handle multiple keys independently", async () => {
    await storage.set("key1", "value1");
    await storage.set("key2", "value2");

    expect(await storage.get("key1")).toBe("value1");
    expect(await storage.get("key2")).toBe("value2");

    await storage.remove("key1");
    expect(await storage.get("key1")).toBeNull();
    expect(await storage.get("key2")).toBe("value2");
  });

  it("should handle empty string values", async () => {
    await storage.set("key1", "");
    const result = await storage.get("key1");
    expect(result).toBe("");
  });

  it("should handle JSON strings", async () => {
    const data = {foo: "bar", num: 42};
    await storage.set("json", JSON.stringify(data));
    const result = await storage.get("json");
    expect(JSON.parse(result!)).toEqual(data);
  });
});

describe("LocalStorageProvider", () => {
  // Note: LocalStorageProvider requires a browser environment
  // These tests verify the interface but may not work in Node/Bun without mocking

  it("should implement StorageProvider interface", () => {
    const storage = new LocalStorageProvider();
    expect(typeof storage.get).toBe("function");
    expect(typeof storage.set).toBe("function");
    expect(typeof storage.remove).toBe("function");
  });
});
