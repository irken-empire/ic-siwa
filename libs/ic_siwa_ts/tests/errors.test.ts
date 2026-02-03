/**
 * Tests for error handling
 */

import {describe, it, expect} from "bun:test";
import {SiwaError, SiwaErrorCode} from "../src/errors";

describe("SiwaError", () => {
  it("should create error with code and message", () => {
    const error = new SiwaError(
      SiwaErrorCode.InvalidSignature,
      "Bad signature"
    );

    expect(error).toBeInstanceOf(Error);
    expect(error).toBeInstanceOf(SiwaError);
    expect(error.code).toBe(SiwaErrorCode.InvalidSignature);
    expect(error.message).toBe("Bad signature");
    expect(error.name).toBe("SiwaError");
  });

  it("should include optional details", () => {
    const details = {foo: "bar"};
    const error = new SiwaError(
      SiwaErrorCode.CanisterError,
      "Canister failed",
      details
    );

    expect(error.details).toEqual(details);
  });

  it("should create error from canister error", () => {
    const canisterError = new Error("Canister rejected the call");
    const error = SiwaError.fromCanisterError(canisterError);

    expect(error.code).toBe(SiwaErrorCode.CanisterError);
    expect(error.message).toBe("Canister rejected the call");
  });

  it("should handle non-Error objects in fromCanisterError", () => {
    const error = SiwaError.fromCanisterError("string error");

    expect(error.code).toBe(SiwaErrorCode.CanisterError);
    expect(error.message).toBe("Unknown canister error");
  });

  it("should handle null in fromCanisterError", () => {
    const error = SiwaError.fromCanisterError(null);

    expect(error.code).toBe(SiwaErrorCode.CanisterError);
    expect(error.message).toBe("Unknown canister error");
  });
});

describe("SiwaErrorCode", () => {
  it("should have all expected error codes", () => {
    expect(SiwaErrorCode.InvalidSignature).toBeDefined();
    expect(SiwaErrorCode.MessageExpired).toBeDefined();
    expect(SiwaErrorCode.InvalidAddress).toBeDefined();
    expect(SiwaErrorCode.CanisterError).toBeDefined();
    expect(SiwaErrorCode.NotAuthenticated).toBeDefined();
    expect(SiwaErrorCode.StorageError).toBeDefined();
  });

  it("should have unique values", () => {
    const codes = Object.values(SiwaErrorCode);
    const uniqueCodes = new Set(codes);
    expect(uniqueCodes.size).toBe(codes.length);
  });
});
