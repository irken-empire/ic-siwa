/**
 * SIWA Error codes and error class
 */

/**
 * Error codes for SIWA operations
 */
export enum SiwaErrorCode {
  /** Operation not yet implemented */
  NotImplemented = "NOT_IMPLEMENTED",
  /** User is not authenticated */
  NotAuthenticated = "NOT_AUTHENTICATED",
  /** Invalid Avalanche address */
  InvalidAddress = "INVALID_ADDRESS",
  /** Invalid signature */
  InvalidSignature = "INVALID_SIGNATURE",
  /** Message has expired */
  MessageExpired = "MESSAGE_EXPIRED",
  /** Session has expired */
  SessionExpired = "SESSION_EXPIRED",
  /** Domain not allowed */
  DomainNotAllowed = "DOMAIN_NOT_ALLOWED",
  /** Network error */
  NetworkError = "NETWORK_ERROR",
  /** Canister error */
  CanisterError = "CANISTER_ERROR",
  /** Invalid input parameter */
  InvalidInput = "INVALID_INPUT",
  /** Storage error */
  StorageError = "STORAGE_ERROR",
  /** Unknown error */
  Unknown = "UNKNOWN",
}

/**
 * Custom error class for SIWA operations
 */
export class SiwaError extends Error {
  readonly code: SiwaErrorCode;
  readonly details?: unknown;

  constructor(code: SiwaErrorCode, message: string, details?: unknown) {
    super(message);
    this.name = "SiwaError";
    this.code = code;
    this.details = details;

    // Maintain proper stack trace for where error was thrown
    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, SiwaError);
    }
  }

  /**
   * Prefix-to-error-code mapping for canister error classification.
   *
   * Order matters: more specific prefixes must come before generic ones
   * (e.g. "Session has expired" before "Session not found").
   */
  private static readonly CANISTER_ERROR_PREFIXES: ReadonlyArray<
    readonly [string, SiwaErrorCode]
  > = [
    // InvalidAddress
    ["Invalid address: ", SiwaErrorCode.InvalidAddress],
    // InvalidSignature
    ["Invalid signature: ", SiwaErrorCode.InvalidSignature],
    ["Signature verification failed: ", SiwaErrorCode.InvalidSignature],
    ["Recovered address ", SiwaErrorCode.InvalidSignature],
    // MessageExpired
    ["Message expired", SiwaErrorCode.MessageExpired],
    ["SIWA message has expired", SiwaErrorCode.MessageExpired],
    ["Login session has expired", SiwaErrorCode.MessageExpired],
    // SessionExpired (must precede "Session not found" check)
    ["Session has expired", SiwaErrorCode.SessionExpired],
    // DomainNotAllowed
    ["Domain not allowed: ", SiwaErrorCode.DomainNotAllowed],
    ["Domain '", SiwaErrorCode.DomainNotAllowed],
    // NotAuthenticated
    ["Session not found", SiwaErrorCode.NotAuthenticated],
    ["No authenticated session", SiwaErrorCode.NotAuthenticated],
    ["Anonymous callers", SiwaErrorCode.NotAuthenticated],
  ];

  /**
   * Classify a canister error message string into the appropriate SiwaErrorCode.
   *
   * Matches the message against known prefix patterns from the canister's error
   * reference. Unrecognised messages fall back to `SiwaErrorCode.CanisterError`.
   */
  private static classifyCanisterError(message: string): SiwaErrorCode {
    for (const [prefix, code] of SiwaError.CANISTER_ERROR_PREFIXES) {
      if (message.startsWith(prefix)) {
        return code;
      }
    }
    return SiwaErrorCode.CanisterError;
  }

  /**
   * Create error from canister response.
   *
   * Accepts an Error instance, a plain string, or an `{Err: string}` response
   * object. The error message is matched against known canister error prefixes
   * to assign a specific `SiwaErrorCode` for programmatic handling.
   */
  static fromCanisterError(error: unknown): SiwaError {
    let message: string;

    if (typeof error === "string") {
      message = error;
    } else if (error instanceof Error) {
      message = error.message;
    } else if (
      error !== null &&
      typeof error === "object" &&
      "Err" in error &&
      typeof (error as {Err: unknown}).Err === "string"
    ) {
      message = (error as {Err: string}).Err;
    } else {
      message = "Unknown canister error";
    }

    const code = SiwaError.classifyCanisterError(message);
    return new SiwaError(code, message, error);
  }

  /**
   * Create error from network failure
   */
  static fromNetworkError(error: unknown): SiwaError {
    const message =
      error instanceof Error ? error.message : "Network request failed";
    return new SiwaError(SiwaErrorCode.NetworkError, message, error);
  }
}
