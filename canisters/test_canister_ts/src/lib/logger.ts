/**
 * IC-SIWA Test Canister - Logging Module
 *
 * Provides structured logging with levels, context tags, and diagnostic export.
 */

export type LogLevel = "debug" | "info" | "warn" | "error";

export interface LogEntry {
  timestamp: Date;
  level: LogLevel;
  context: string;
  message: string;
  data?: unknown;
  error?: Error;
}

export interface LoggerConfig {
  /** Minimum log level to output (default: "debug") */
  minLevel: LogLevel;
  /** Whether to include timestamps in console output (default: true) */
  showTimestamp: boolean;
  /** Maximum number of log entries to keep in memory (default: 500) */
  maxEntries: number;
  /** Whether logging is enabled (default: true) */
  enabled: boolean;
}

const LOG_LEVEL_ORDER: Record<LogLevel, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
};

const LOG_LEVEL_STYLES: Record<LogLevel, string> = {
  debug: "color: #6B7280; font-weight: normal;",
  info: "color: #3B82F6; font-weight: normal;",
  warn: "color: #F59E0B; font-weight: bold;",
  error: "color: #EF4444; font-weight: bold;",
};

const CONTEXT_STYLES: Record<string, string> = {
  WALLET: "color: #8B5CF6; font-weight: bold;",
  SIWA: "color: #10B981; font-weight: bold;",
  NETWORK: "color: #F59E0B; font-weight: bold;",
  UI: "color: #EC4899; font-weight: bold;",
  EIP6963: "color: #06B6D4; font-weight: bold;",
  WC: "color: #3B99FC; font-weight: bold;",
  DEFAULT: "color: #6B7280; font-weight: bold;",
};

class Logger {
  private config: LoggerConfig;
  private entries: LogEntry[] = [];
  private context: string;

  constructor(context: string, config?: Partial<LoggerConfig>) {
    this.context = context;
    this.config = {
      minLevel: "debug",
      showTimestamp: true,
      maxEntries: 500,
      enabled: true,
      ...config,
    };
  }

  private shouldLog(level: LogLevel): boolean {
    if (!this.config.enabled) return false;
    return LOG_LEVEL_ORDER[level] >= LOG_LEVEL_ORDER[this.config.minLevel];
  }

  private formatTimestamp(date: Date): string {
    return date.toISOString().slice(11, 23); // HH:mm:ss.sss
  }

  private getContextStyle(): string {
    return CONTEXT_STYLES[this.context] || CONTEXT_STYLES.DEFAULT;
  }

  private log(
    level: LogLevel,
    message: string,
    data?: unknown,
    error?: Error
  ): void {
    const entry: LogEntry = {
      timestamp: new Date(),
      level,
      context: this.context,
      message,
      data,
      error,
    };

    // Store entry locally
    this.entries.push(entry);
    if (this.entries.length > this.config.maxEntries) {
      this.entries.shift();
    }

    // Also add to global store for exportAllLogs()
    addToGlobalStore(entry);

    // Check if we should output
    if (!this.shouldLog(level)) return;

    // Build console output
    const timestamp = this.config.showTimestamp
      ? `${this.formatTimestamp(entry.timestamp)} `
      : "";
    const prefix = `%c${timestamp}%c[${this.context}]%c [${level.toUpperCase()}]`;

    const styles = [
      "color: #6B7280;", // timestamp
      this.getContextStyle(), // context
      LOG_LEVEL_STYLES[level], // level
    ];

    // Output to console
    if (error) {
      console.groupCollapsed(prefix, ...styles, message);
      if (data !== undefined) {
        console.log("Data:", data);
      }
      console.error("Error:", error);
      if (error.stack) {
        console.log("Stack:", error.stack);
      }
      console.groupEnd();
    } else if (data !== undefined) {
      console.groupCollapsed(prefix, ...styles, message);
      console.log("Data:", data);
      console.groupEnd();
    } else {
      console.log(prefix, ...styles, message);
    }
  }

  debug(message: string, data?: unknown): void {
    this.log("debug", message, data);
  }

  info(message: string, data?: unknown): void {
    this.log("info", message, data);
  }

  warn(message: string, data?: unknown): void {
    this.log("warn", message, data);
  }

  error(message: string, error?: Error | unknown, data?: unknown): void {
    const err = error instanceof Error ? error : undefined;
    const extraData = error instanceof Error ? data : error;
    this.log("error", message, extraData, err);
  }

  /** Get all stored log entries */
  getEntries(): LogEntry[] {
    return [...this.entries];
  }

  /** Get entries filtered by level */
  getEntriesByLevel(level: LogLevel): LogEntry[] {
    return this.entries.filter((e) => e.level === level);
  }

  /** Clear all stored entries */
  clear(): void {
    this.entries = [];
  }

  /** Export logs as JSON string */
  export(): string {
    return JSON.stringify(
      this.entries.map((e) => ({
        ...e,
        timestamp: e.timestamp.toISOString(),
        error: e.error
          ? {
              name: e.error.name,
              message: e.error.message,
              stack: e.error.stack,
            }
          : undefined,
      })),
      jsonReplacer,
      2
    );
  }

  /** Create a child logger with a sub-context */
  child(subContext: string): Logger {
    return new Logger(`${this.context}:${subContext}`, this.config);
  }
}

// Global log storage for cross-context export
const globalLogStore: LogEntry[] = [];
const MAX_GLOBAL_ENTRIES = 1000;

function addToGlobalStore(entry: LogEntry): void {
  globalLogStore.push(entry);
  if (globalLogStore.length > MAX_GLOBAL_ENTRIES) {
    globalLogStore.shift();
  }
}

/**
 * JSON replacer function that handles BigInt and other non-serializable types
 */
function jsonReplacer(_key: string, value: unknown): unknown {
  if (typeof value === "bigint") {
    return value.toString() + "n"; // Append 'n' to indicate it was a BigInt
  }
  if (value instanceof Uint8Array) {
    return Array.from(value); // Convert Uint8Array to regular array
  }
  if (ArrayBuffer.isView(value)) {
    return Array.from(new Uint8Array(value.buffer)); // Handle other typed arrays
  }
  return value;
}

/** Create a logger instance for a specific context */
export function createLogger(
  context: string,
  config?: Partial<LoggerConfig>
): Logger {
  return new Logger(context, config);
}

/** Export all global logs */
export function exportAllLogs(): string {
  return JSON.stringify(
    globalLogStore.map((e) => ({
      ...e,
      timestamp: e.timestamp.toISOString(),
      error: e.error
        ? {
            name: e.error.name,
            message: e.error.message,
            stack: e.error.stack,
          }
        : undefined,
    })),
    jsonReplacer,
    2
  );
}

/** Clear all global logs */
export function clearAllLogs(): void {
  globalLogStore.length = 0;
}

/** Get global log count by level */
export function getLogCounts(): Record<LogLevel, number> {
  return {
    debug: globalLogStore.filter((e) => e.level === "debug").length,
    info: globalLogStore.filter((e) => e.level === "info").length,
    warn: globalLogStore.filter((e) => e.level === "warn").length,
    error: globalLogStore.filter((e) => e.level === "error").length,
  };
}

// Pre-configured loggers for common contexts
export const walletLog = createLogger("WALLET");
export const siwaLog = createLogger("SIWA");
export const networkLog = createLogger("NETWORK");
export const uiLog = createLogger("UI");
export const eip6963Log = createLogger("EIP6963");
export const wcLog = createLogger("WC");
