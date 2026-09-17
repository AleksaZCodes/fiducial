/**
 * Storage contract — object storage: put, get, delete, list, signed URLs.
 *
 * Mirrors `fiducial_adapters::storage` in Rust. Designed against R2, S3,
 * and Supabase Storage. Vendor-specific features stay on the vendor type.
 */

import {
  createClient as createSupabaseClient,
  type SupabaseClient,
} from "@supabase/supabase-js";

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
  // Accepts and ignores `env` so every vendor class in this contract shares
  // one constructor shape for the generated factory to call uniformly.
  constructor(_env?: unknown) {}

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

// ── R2 ───────────────────────────────────────────────────────────────────

/**
 * R2 — Cloudflare's object storage, reached through a binding (`env.BUCKET`)
 * inside a Worker. No egress fees, which is why `docs/specs/…cloudflare-default`
 * names it as the default for firmware distribution.
 *
 * `R2Binding` mirrors the methods this adapter calls from
 * `@cloudflare/workers-types`' `R2Bucket`, without depending on that package
 * at the type level — same reasoning as `D1Binding` above.
 *
 * **Binding convention:** `env.BUCKET`, for the same reason `D1Database`
 * reads `env.DB` — see its doc comment.
 *
 * **`signedUrl` is not implemented, on purpose.** A time-limited URL for an
 * R2 object requires AWS SigV4 signing against R2's S3-compatible API,
 * which needs an R2 API token (access key + secret) — credentials the
 * binding does not carry and that only exist outside the Workers runtime.
 * Producing one from inside a Worker is a second, S3-shaped client this
 * adapter does not build, because nothing in this repository has needed a
 * signed URL yet. Calling it throws a `StorageError` that says so, rather
 * than returning an empty string like `NoneStorage` — a silently-empty URL
 * from a *selected* vendor would look like a bug in R2, not an
 * unimplemented method.
 */
interface R2Binding {
  put(
    key: string,
    value: Uint8Array,
    options?: { httpMetadata?: { contentType?: string } },
  ): Promise<unknown>;
  get(key: string): Promise<R2Object | null>;
  delete(key: string): Promise<void>;
  list(options?: {
    prefix?: string;
    cursor?: string;
  }): Promise<{ objects: { key: string }[]; truncated: boolean; cursor?: string }>;
}

interface R2Object {
  arrayBuffer(): Promise<ArrayBuffer>;
}

export class R2Storage implements Storage {
  private readonly bucket: R2Binding;

  constructor(env: { BUCKET?: R2Binding }) {
    if (!env?.BUCKET) {
      throw new StorageError(
        "R2Storage: no `BUCKET` binding on env — add a [[r2_buckets]] block " +
          'with `binding = "BUCKET"` to wrangler.toml',
      );
    }
    this.bucket = env.BUCKET;
  }

  async put(
    key: string,
    bytes: Uint8Array,
    contentType?: string,
  ): Promise<void> {
    try {
      await this.bucket.put(key, bytes, {
        httpMetadata: contentType ? { contentType } : undefined,
      });
    } catch (err) {
      throw new StorageError(`R2 put failed: ${String(err)}`, key);
    }
  }

  async get(key: string): Promise<Uint8Array | null> {
    try {
      const obj = await this.bucket.get(key);
      if (!obj) return null;
      return new Uint8Array(await obj.arrayBuffer());
    } catch (err) {
      throw new StorageError(`R2 get failed: ${String(err)}`, key);
    }
  }

  async delete(key: string): Promise<void> {
    try {
      await this.bucket.delete(key);
    } catch (err) {
      throw new StorageError(`R2 delete failed: ${String(err)}`, key);
    }
  }

  async list(prefix: string): Promise<string[]> {
    try {
      const keys: string[] = [];
      let cursor: string | undefined;
      do {
        const page = await this.bucket.list({ prefix, cursor });
        keys.push(...page.objects.map((o) => o.key));
        cursor = page.truncated ? page.cursor : undefined;
      } while (cursor);
      return keys;
    } catch (err) {
      throw new StorageError(`R2 list failed: ${String(err)}`);
    }
  }

  async signedUrl(key: string, _ttlSeconds: number): Promise<string> {
    throw new StorageError(
      "R2Storage.signedUrl: not implemented — R2 presigned URLs require " +
        "SigV4 signing against the S3-compatible API with an R2 API token, " +
        "which the Workers binding does not carry. See the class doc comment.",
      key,
    );
  }
}

// ── Supabase Storage ──────────────────────────────────────────────────────────

/**
 * Supabase Storage — Supabase's managed object storage, reached over HTTPS
 * using the Supabase JS client with a service-role key.
 *
 * **Credentials:** `SUPABASE_URL` (your project URL) and
 * `SUPABASE_SERVICE_ROLE_KEY` (service-role key, bypasses RLS for
 * server-side writes). Set both with `wrangler secret put`.
 *
 * **Bucket:** reads `SUPABASE_STORAGE_BUCKET` from env, defaulting to
 * `"assets"`. Create the bucket in the Supabase dashboard before use.
 *
 * **`list` traverses sub-folders.** `prefix` is treated as a path prefix:
 * files whose full path starts with `prefix` are returned via breadth-first
 * traversal. A trailing `/` in `prefix` is stripped before dispatch.
 *
 * **`signedUrl` is fully implemented.** Supabase Storage's native
 * `createSignedUrl` endpoint is used; `ttlSeconds` is passed as-is.
 */
export class SupabaseStorage implements Storage {
  private readonly client: SupabaseClient;
  private readonly bucket: string;

  constructor(env: {
    SUPABASE_URL?: string;
    SUPABASE_SERVICE_ROLE_KEY?: string;
    SUPABASE_STORAGE_BUCKET?: string;
  }) {
    if (!env.SUPABASE_URL || !env.SUPABASE_SERVICE_ROLE_KEY) {
      throw new StorageError(
        "SupabaseStorage: SUPABASE_URL and SUPABASE_SERVICE_ROLE_KEY are required — " +
          "set both with `wrangler secret put`",
      );
    }
    this.client = createSupabaseClient(
      env.SUPABASE_URL,
      env.SUPABASE_SERVICE_ROLE_KEY,
      { auth: { persistSession: false, autoRefreshToken: false } },
    );
    this.bucket = env.SUPABASE_STORAGE_BUCKET ?? "assets";
  }

  async put(
    key: string,
    bytes: Uint8Array,
    contentType?: string,
  ): Promise<void> {
    const { error } = await this.client.storage.from(this.bucket).upload(key, bytes, {
      contentType: contentType ?? "application/octet-stream",
      upsert: true,
    });
    if (error) {
      throw new StorageError(`SupabaseStorage put failed: ${error.message}`, key);
    }
  }

  async get(key: string): Promise<Uint8Array | null> {
    const { data, error } = await this.client.storage
      .from(this.bucket)
      .download(key);
    if (error) {
      if (
        error.message.includes("Not Found") ||
        error.message.includes("does not exist") ||
        error.message.includes("Object not found")
      ) {
        return null;
      }
      throw new StorageError(`SupabaseStorage get failed: ${error.message}`, key);
    }
    return new Uint8Array(await data.arrayBuffer());
  }

  async delete(key: string): Promise<void> {
    const { error } = await this.client.storage.from(this.bucket).remove([key]);
    if (error) {
      throw new StorageError(`SupabaseStorage delete failed: ${error.message}`, key);
    }
  }

  async list(prefix: string): Promise<string[]> {
    // Normalise: strip trailing slash, treat "" as root.
    const folder = prefix.replace(/\/$/, "");
    return this._listFolder(folder, folder ? `${folder}/` : "");
  }

  private async _listFolder(path: string, keyPrefix: string): Promise<string[]> {
    const keys: string[] = [];
    const limit = 1000;
    let offset = 0;

    for (;;) {
      const { data, error } = await this.client.storage
        .from(this.bucket)
        .list(path || undefined, { limit, offset });

      if (error) {
        throw new StorageError(`SupabaseStorage list failed: ${error.message}`);
      }
      if (!data || data.length === 0) break;

      for (const item of data) {
        const fullKey = `${keyPrefix}${item.name}`;
        if (item.id !== null) {
          // Real file.
          keys.push(fullKey);
        } else {
          // Implicit folder — recurse.
          const sub = path ? `${path}/${item.name}` : item.name;
          const subKeys = await this._listFolder(sub, `${fullKey}/`);
          keys.push(...subKeys);
        }
      }

      offset += data.length;
      if (data.length < limit) break;
    }

    return keys;
  }

  async signedUrl(key: string, ttlSeconds: number): Promise<string> {
    const { data, error } = await this.client.storage
      .from(this.bucket)
      .createSignedUrl(key, ttlSeconds);
    if (error) {
      throw new StorageError(
        `SupabaseStorage signedUrl failed: ${error.message}`,
        key,
      );
    }
    return data.signedUrl;
  }
}
