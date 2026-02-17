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
   * Create error from canister response
   */
  static fromCanisterError(error: unknown): SiwaError {
    const message =
      error instanceof Error ? error.message : "Unknown canister error";
    return new SiwaError(SiwaErrorCode.CanisterError, message, error);
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
