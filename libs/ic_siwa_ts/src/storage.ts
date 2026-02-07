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

  constructor(prefix = "ic_siwa_") {
    this.prefix = prefix;
  }

  async get(key: string): Promise<string | null> {
    if (typeof localStorage === "undefined") {
      return null;
    }
    return localStorage.getItem(this.prefix + key);
  }

  async set(key: string, value: string): Promise<void> {
    if (typeof localStorage === "undefined") {
      return;
    }
    localStorage.setItem(this.prefix + key, value);
  }

  async remove(key: string): Promise<void> {
    if (typeof localStorage === "undefined") {
      return;
    }
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

  constructor(prefix = "ic_siwa_") {
    this.prefix = prefix;
  }

  async get(key: string): Promise<string | null> {
    if (typeof sessionStorage === "undefined") {
      return null;
    }
    return sessionStorage.getItem(this.prefix + key);
  }

  async set(key: string, value: string): Promise<void> {
    if (typeof sessionStorage === "undefined") {
      return;
    }
    sessionStorage.setItem(this.prefix + key, value);
  }

  async remove(key: string): Promise<void> {
    if (typeof sessionStorage === "undefined") {
      return;
    }
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
