/**
 * Storage providers for caching identity
 */

/**
 * Storage provider interface
 */
export interface StorageProvider {
  /** Get a value by key */
  get(key: string): Promise<string | null>;
  /** Set a value by key */
  set(key: string, value: string): Promise<void>;
  /** Remove a value by key */
  remove(key: string): Promise<void>;
}

/**
 * LocalStorage-based storage provider (browser)
 */
export class LocalStorageProvider implements StorageProvider {
  private prefix: string;
  private warned = false;

  constructor(prefix = "ic_siwa_") {
    this.prefix = prefix;
  }

  private warnIfUnavailable(): boolean {
    if (typeof localStorage === "undefined") {
      if (!this.warned) {
        console.warn(
          "[ic-siwa] localStorage is not available. Session will not persist. " +
            "Use MemoryStorageProvider or a custom StorageProvider for non-browser environments."
        );
        this.warned = true;
      }
      return true;
    }
    return false;
  }

  async get(key: string): Promise<string | null> {
    if (this.warnIfUnavailable()) return null;
    return localStorage.getItem(this.prefix + key);
  }

  async set(key: string, value: string): Promise<void> {
    if (this.warnIfUnavailable()) return;
    localStorage.setItem(this.prefix + key, value);
  }

  async remove(key: string): Promise<void> {
    if (this.warnIfUnavailable()) return;
    localStorage.removeItem(this.prefix + key);
  }
}

/**
 * SessionStorage-based storage provider (browser)
 *
 * Uses sessionStorage which is cleared when the browser tab is closed.
 * This is more secure than localStorage for storing session keys since
 * sensitive key material does not persist beyond the browser session.
 */
export class SessionStorageProvider implements StorageProvider {
  private prefix: string;
  private warned = false;

  constructor(prefix = "ic_siwa_") {
    this.prefix = prefix;
  }

  private warnIfUnavailable(): boolean {
    if (typeof sessionStorage === "undefined") {
      if (!this.warned) {
        console.warn(
          "[ic-siwa] sessionStorage is not available. Session will not persist. " +
            "Use MemoryStorageProvider or a custom StorageProvider for non-browser environments."
        );
        this.warned = true;
      }
      return true;
    }
    return false;
  }

  async get(key: string): Promise<string | null> {
    if (this.warnIfUnavailable()) return null;
    return sessionStorage.getItem(this.prefix + key);
  }

  async set(key: string, value: string): Promise<void> {
    if (this.warnIfUnavailable()) return;
    sessionStorage.setItem(this.prefix + key, value);
  }

  async remove(key: string): Promise<void> {
    if (this.warnIfUnavailable()) return;
    sessionStorage.removeItem(this.prefix + key);
  }
}

/**
 * In-memory storage provider (for testing or SSR)
 */
export class MemoryStorageProvider implements StorageProvider {
  private storage = new Map<string, string>();

  async get(key: string): Promise<string | null> {
    return this.storage.get(key) ?? null;
  }

  async set(key: string, value: string): Promise<void> {
    this.storage.set(key, value);
  }

  async remove(key: string): Promise<void> {
    this.storage.delete(key);
  }

  /** Clear all stored values */
  clear(): void {
    this.storage.clear();
  }
}
