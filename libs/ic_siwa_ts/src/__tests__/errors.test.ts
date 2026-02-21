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
  it("wraps Error instance and classifies by message", () => {
    const original = new Error("canister trapped");
    const error = SiwaError.fromCanisterError(original);
    // "canister trapped" doesn't match any known prefix → CanisterError
    expect(error.code).toBe(SiwaErrorCode.CanisterError);
    expect(error.message).toBe("canister trapped");
    expect(error.details).toBe(original);
  });

  it("handles plain string input", () => {
    const error = SiwaError.fromCanisterError("some error string");
    expect(error.code).toBe(SiwaErrorCode.CanisterError);
    expect(error.message).toBe("some error string");
    expect(error.details).toBe("some error string");
  });

  it("handles {Err: string} response objects", () => {
    const response = {Err: "Invalid address: bad checksum"};
    const error = SiwaError.fromCanisterError(response);
    expect(error.code).toBe(SiwaErrorCode.InvalidAddress);
    expect(error.message).toBe("Invalid address: bad checksum");
    expect(error.details).toBe(response);
  });

  it("handles non-Error, non-string input", () => {
    const error = SiwaError.fromCanisterError(42);
    expect(error.code).toBe(SiwaErrorCode.CanisterError);
    expect(error.message).toBe("Unknown canister error");
  });

  it("handles null input", () => {
    const error = SiwaError.fromCanisterError(null);
    expect(error.code).toBe(SiwaErrorCode.CanisterError);
    expect(error.message).toBe("Unknown canister error");
  });
});

describe("SiwaError.fromCanisterError classification", () => {
  // InvalidAddress
  it("classifies 'Invalid address: ...' as InvalidAddress", () => {
    const error = SiwaError.fromCanisterError(
      "Invalid address: missing 0x prefix"
    );
    expect(error.code).toBe(SiwaErrorCode.InvalidAddress);
  });

  // InvalidSignature
  it("classifies 'Invalid signature: ...' as InvalidSignature", () => {
    const error = SiwaError.fromCanisterError(
      "Invalid signature: recovery failed"
    );
    expect(error.code).toBe(SiwaErrorCode.InvalidSignature);
  });

  it("classifies 'Signature verification failed: ...' as InvalidSignature", () => {
    const error = SiwaError.fromCanisterError(
      "Signature verification failed: bad v value"
    );
    expect(error.code).toBe(SiwaErrorCode.InvalidSignature);
  });

  it("classifies 'Recovered address ...' as InvalidSignature", () => {
    const error = SiwaError.fromCanisterError(
      "Recovered address 0xabc does not match expected address 0xdef"
    );
    expect(error.code).toBe(SiwaErrorCode.InvalidSignature);
  });

  // MessageExpired
  it("classifies 'Message expired' as MessageExpired", () => {
    const error = SiwaError.fromCanisterError("Message expired");
    expect(error.code).toBe(SiwaErrorCode.MessageExpired);
  });

  it("classifies 'SIWA message has expired' as MessageExpired", () => {
    const error = SiwaError.fromCanisterError("SIWA message has expired");
    expect(error.code).toBe(SiwaErrorCode.MessageExpired);
  });

  it("classifies 'Login session has expired' as MessageExpired", () => {
    const error = SiwaError.fromCanisterError("Login session has expired");
    expect(error.code).toBe(SiwaErrorCode.MessageExpired);
  });

  // SessionExpired
  it("classifies 'Session has expired' as SessionExpired", () => {
    const error = SiwaError.fromCanisterError("Session has expired");
    expect(error.code).toBe(SiwaErrorCode.SessionExpired);
  });

  // DomainNotAllowed
  it("classifies 'Domain not allowed: ...' as DomainNotAllowed", () => {
    const error = SiwaError.fromCanisterError("Domain not allowed: evil.com");
    expect(error.code).toBe(SiwaErrorCode.DomainNotAllowed);
  });

  it("classifies \"Domain '...'\" as DomainNotAllowed", () => {
    const error = SiwaError.fromCanisterError(
      "Domain 'evil.com' is not in the allowed domains list"
    );
    expect(error.code).toBe(SiwaErrorCode.DomainNotAllowed);
  });

  // NotAuthenticated
  it("classifies 'Session not found' as NotAuthenticated", () => {
    const error = SiwaError.fromCanisterError("Session not found");
    expect(error.code).toBe(SiwaErrorCode.NotAuthenticated);
  });

  it("classifies 'No authenticated session...' as NotAuthenticated", () => {
    const error = SiwaError.fromCanisterError(
      "No authenticated session found for address 0x123"
    );
    expect(error.code).toBe(SiwaErrorCode.NotAuthenticated);
  });

  it("classifies 'Anonymous callers...' as NotAuthenticated", () => {
    const error = SiwaError.fromCanisterError(
      "Anonymous callers are not allowed"
    );
    expect(error.code).toBe(SiwaErrorCode.NotAuthenticated);
  });

  // Fallback
  it("falls back to CanisterError for unrecognised messages", () => {
    const error = SiwaError.fromCanisterError(
      "Some completely unknown canister error"
    );
    expect(error.code).toBe(SiwaErrorCode.CanisterError);
  });

  it("falls back to CanisterError when wrapping Error with unknown message", () => {
    const error = SiwaError.fromCanisterError(
      new Error("unexpected internal failure")
    );
    expect(error.code).toBe(SiwaErrorCode.CanisterError);
  });

  it("classifies Error with known message prefix", () => {
    const error = SiwaError.fromCanisterError(
      new Error("Invalid address: wrong length")
    );
    expect(error.code).toBe(SiwaErrorCode.InvalidAddress);
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
