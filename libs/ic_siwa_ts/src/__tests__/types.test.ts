import {describe, expect, it} from "bun:test";
import {
  SiwaClient,
  SiwaError,
  SiwaErrorCode,
  LocalStorageProvider,
  MemoryStorageProvider,
  generateSessionKey,
  VERSION,
} from "../index";
import {idlFactory, init} from "../candid";

describe("Package exports", () => {
  it("should export SiwaClient", () => {
    expect(SiwaClient).toBeDefined();
    expect(typeof SiwaClient).toBe("function");
  });

  it("should export SiwaError and SiwaErrorCode", () => {
    expect(SiwaError).toBeDefined();
    expect(SiwaErrorCode).toBeDefined();
    expect(typeof SiwaError).toBe("function");
    expect(typeof SiwaErrorCode).toBe("object");
  });

  it("should export storage providers", () => {
    expect(LocalStorageProvider).toBeDefined();
    expect(MemoryStorageProvider).toBeDefined();
    expect(typeof LocalStorageProvider).toBe("function");
    expect(typeof MemoryStorageProvider).toBe("function");
  });

  it("should export generateSessionKey", () => {
    expect(generateSessionKey).toBeDefined();
    expect(typeof generateSessionKey).toBe("function");
  });

  it("should export VERSION", () => {
    expect(VERSION).toBeDefined();
    expect(typeof VERSION).toBe("string");
    expect(VERSION).toMatch(/^\d+\.\d+\.\d+$/);
  });
});

describe("Candid exports", () => {
  it("should export idlFactory", () => {
    expect(idlFactory).toBeDefined();
    expect(typeof idlFactory).toBe("function");
  });

  it("should export init", () => {
    expect(init).toBeDefined();
    expect(typeof init).toBe("function");
  });

  it("idlFactory should return service definition when called", () => {
    // Mock IDL object with required methods
    const mockIDL = {
      Record: () => ({}),
      Text: {},
      Nat64: {},
      Opt: () => ({}),
      Vec: () => ({}),
      Principal: {},
      Nat8: {},
      Variant: () => ({}),
      Func: () => ({}),
      Service: (methods: unknown) => methods,
    };

    const result = idlFactory({IDL: mockIDL});
    expect(result).toBeDefined();
    expect(typeof result).toBe("object");
  });

  it("init should return init args when called", () => {
    const mockIDL = {
      Record: () => ({}),
      Text: {},
      Nat64: {},
      Opt: () => ({}),
      Vec: () => ({}),
      Principal: {},
    };

    const result = init({IDL: mockIDL});
    expect(result).toBeDefined();
    expect(Array.isArray(result)).toBe(true);
    expect(result.length).toBe(1);
  });
});
