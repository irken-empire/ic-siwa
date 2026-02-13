import {describe, expect, it} from "bun:test";
import {SiwaError, SiwaErrorCode} from "../errors";

describe("SiwaError", () => {
  it("creates error with code and message", () => {
    const error = new SiwaError(SiwaErrorCode.InvalidAddress, "Bad address");
    expect(error.code).toBe(SiwaErrorCode.InvalidAddress);
    expect(error.message).toBe("Bad address");
    expect(error.name).toBe("SiwaError");
    expect(error.details).toBeUndefined();
  });

  it("creates error with details", () => {
    const details = {field: "address", value: "0x123"};
    const error = new SiwaError(
      SiwaErrorCode.InvalidAddress,
      "Bad address",
      details
    );
    expect(error.details).toEqual(details);
  });

  it("is instanceof Error", () => {
    const error = new SiwaError(SiwaErrorCode.Unknown, "test");
    expect(error).toBeInstanceOf(Error);
    expect(error).toBeInstanceOf(SiwaError);
  });

  it("has stack trace", () => {
    const error = new SiwaError(SiwaErrorCode.Unknown, "test");
    expect(error.stack).toBeDefined();
  });
});

describe("SiwaError.fromCanisterError", () => {
  it("wraps Error instance", () => {
    const original = new Error("canister trapped");
    const error = SiwaError.fromCanisterError(original);
    expect(error.code).toBe(SiwaErrorCode.CanisterError);
    expect(error.message).toBe("canister trapped");
    expect(error.details).toBe(original);
  });

  it("handles non-Error input", () => {
    const error = SiwaError.fromCanisterError("string error");
    expect(error.code).toBe(SiwaErrorCode.CanisterError);
    expect(error.message).toBe("Unknown canister error");
    expect(error.details).toBe("string error");
  });
});

describe("SiwaError.fromNetworkError", () => {
  it("wraps Error instance", () => {
    const original = new Error("fetch failed");
    const error = SiwaError.fromNetworkError(original);
    expect(error.code).toBe(SiwaErrorCode.NetworkError);
    expect(error.message).toBe("fetch failed");
  });

  it("handles non-Error input", () => {
    const error = SiwaError.fromNetworkError(null);
    expect(error.code).toBe(SiwaErrorCode.NetworkError);
    expect(error.message).toBe("Network request failed");
  });
});

describe("SiwaErrorCode", () => {
  it("has all expected codes", () => {
    const s = (v: SiwaErrorCode): string => v;
    expect(s(SiwaErrorCode.NotImplemented)).toBe("NOT_IMPLEMENTED");
    expect(s(SiwaErrorCode.NotAuthenticated)).toBe("NOT_AUTHENTICATED");
    expect(s(SiwaErrorCode.InvalidAddress)).toBe("INVALID_ADDRESS");
    expect(s(SiwaErrorCode.InvalidSignature)).toBe("INVALID_SIGNATURE");
    expect(s(SiwaErrorCode.MessageExpired)).toBe("MESSAGE_EXPIRED");
    expect(s(SiwaErrorCode.SessionExpired)).toBe("SESSION_EXPIRED");
    expect(s(SiwaErrorCode.DomainNotAllowed)).toBe("DOMAIN_NOT_ALLOWED");
    expect(s(SiwaErrorCode.NetworkError)).toBe("NETWORK_ERROR");
    expect(s(SiwaErrorCode.CanisterError)).toBe("CANISTER_ERROR");
    expect(s(SiwaErrorCode.StorageError)).toBe("STORAGE_ERROR");
    expect(s(SiwaErrorCode.Unknown)).toBe("UNKNOWN");
  });
});
