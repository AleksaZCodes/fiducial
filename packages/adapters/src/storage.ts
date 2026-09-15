/**
 * Storage contract — object storage: put, get, delete, list, signed URLs.
 *
 * Mirrors `fiducial_adapters::storage` in Rust. Designed against R2, S3,
 * and Supabase Storage. Vendor-specific features stay on the vendor type.
 */

export class StorageError extends Error {
  constructor(
    message: string,
    public readonly key?: string,
  ) {
    super(message);
    this.name = "StorageError";
  }
}

/** Object storage contract. */
export interface Storage {
  /** Write bytes at `key`, replacing any existing object. */
  put(key: string, bytes: Uint8Array, contentType?: string): Promise<void>;

  /** Read the object at `key`. Returns null when the key does not exist. */
  get(key: string): Promise<Uint8Array | null>;

  /** Delete the object at `key`. Succeeds even if the key does not exist. */
  delete(key: string): Promise<void>;

  /** List all keys with the given prefix. An empty prefix lists everything. */
  list(prefix: string): Promise<string[]>;

  /**
   * Return a time-limited signed URL for `key`.
   *
   * `ttlSeconds` is a hint; vendors may round up to their minimum granularity.
   * The `None` implementation always returns an empty string.
   */
  signedUrl(key: string, ttlSeconds: number): Promise<string>;
}

// ── None implementation ───────────────────────────────────────────────────────

/** No-op object storage — all writes succeed silently; all reads return null. */
export class NoneStorage implements Storage {
  async put(
    _key: string,
    _bytes: Uint8Array,
    _contentType?: string,
  ): Promise<void> {}

  async get(_key: string): Promise<Uint8Array | null> {
    return null;
  }

  async delete(_key: string): Promise<void> {}

  async list(_prefix: string): Promise<string[]> {
    return [];
  }

  async signedUrl(_key: string, _ttlSeconds: number): Promise<string> {
    return "";
  }
}
