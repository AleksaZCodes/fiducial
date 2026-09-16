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
 * SupabaseStorage — Supabase's object storage, reached over HTTPS using the
 * Supabase REST API and a service-role key.
 *
 * **Secret convention.** Reads `env.SUPABASE_URL` and
 * `env.SUPABASE_SERVICE_ROLE_KEY`. The service-role key bypasses Row Level
 * Security, which is correct for server-side storage operations — a product
 * that serves signed URLs does not want row-level constraints on `put`.
 *
 * **Bucket convention.** Reads `env.SUPABASE_STORAGE_BUCKET` (default:
 * `"assets"`). Declare the bucket in Supabase's dashboard or via a migration;
 * this adapter does not create it.
 *
 * **Reachability.** Any HTTPS-capable runtime: Worker, Tauri backend, Node.js,
 * edge functions. Unlike `R2Storage` there is no binding; every call is an
 * authenticated HTTPS request to `${SUPABASE_URL}/storage/v1/object/…`.
 *
 * **`signedUrl` is implemented**, because Supabase Storage provides a REST
 * endpoint for it — unlike R2 which requires SigV4 signing.
 *
 * **`list` paginates internally.** Supabase's `list` endpoint returns at most
 * 100 objects by default; this adapter pages until exhausted.
 *
 * **Why not `@supabase/storage-js`?** The SDK is fine, but it adds a JS
 * dependency that this adapter would be the only consumer of in the adapters
 * package, and all it does is wrap the same REST calls this class makes
 * directly. Adding it is the right call when a second consumer needs it or
 * the REST surface grows complex enough to justify it.
 */
export class SupabaseStorage implements Storage {
  private readonly url: string;
  private readonly key: string;
  private readonly bucket: string;

  constructor(env: {
    SUPABASE_URL?: string;
    SUPABASE_SERVICE_ROLE_KEY?: string;
    SUPABASE_STORAGE_BUCKET?: string;
  }) {
    if (!env?.SUPABASE_URL) {
      throw new StorageError(
        "SupabaseStorage: missing SUPABASE_URL — set it with `wrangler secret put SUPABASE_URL`",
      );
    }
    if (!env?.SUPABASE_SERVICE_ROLE_KEY) {
      throw new StorageError(
        "SupabaseStorage: missing SUPABASE_SERVICE_ROLE_KEY — set it with " +
          "`wrangler secret put SUPABASE_SERVICE_ROLE_KEY`",
      );
    }
    this.url = env.SUPABASE_URL.replace(/\/$/, "");
    this.key = env.SUPABASE_SERVICE_ROLE_KEY;
    this.bucket = env.SUPABASE_STORAGE_BUCKET ?? "assets";
  }

  private objectUrl(key: string): string {
    return `${this.url}/storage/v1/object/${encodeURIComponent(this.bucket)}/${key}`;
  }

  private headers(extra?: Record<string, string>): Record<string, string> {
    return {
      Authorization: `Bearer ${this.key}`,
      apikey: this.key,
      ...extra,
    };
  }

  async put(key: string, bytes: Uint8Array, contentType?: string): Promise<void> {
    try {
      const res = await fetch(this.objectUrl(key), {
        method: "POST",
        headers: this.headers({
          "Content-Type": contentType ?? "application/octet-stream",
          "x-upsert": "true",
        }),
        body: bytes as unknown as BodyInit,
      });
      if (!res.ok) {
        const body = await res.text().catch(() => "");
        throw new StorageError(`SupabaseStorage put failed (${res.status}): ${body}`, key);
      }
    } catch (err) {
      if (err instanceof StorageError) throw err;
      throw new StorageError(`SupabaseStorage put failed: ${String(err)}`, key);
    }
  }

  async get(key: string): Promise<Uint8Array | null> {
    try {
      const res = await fetch(this.objectUrl(key), {
        headers: this.headers(),
      });
      if (res.status === 404) return null;
      if (!res.ok) {
        const body = await res.text().catch(() => "");
        throw new StorageError(`SupabaseStorage get failed (${res.status}): ${body}`, key);
      }
      return new Uint8Array(await res.arrayBuffer());
    } catch (err) {
      if (err instanceof StorageError) throw err;
      throw new StorageError(`SupabaseStorage get failed: ${String(err)}`, key);
    }
  }

  async delete(key: string): Promise<void> {
    try {
      const res = await fetch(
        `${this.url}/storage/v1/object/${encodeURIComponent(this.bucket)}`,
        {
          method: "DELETE",
          headers: this.headers({ "Content-Type": "application/json" }),
          body: JSON.stringify({ prefixes: [key] }),
        },
      );
      // 200 and 400 ("not found") are both success for our contract
      if (!res.ok && res.status !== 400) {
        const body = await res.text().catch(() => "");
        throw new StorageError(`SupabaseStorage delete failed (${res.status}): ${body}`, key);
      }
    } catch (err) {
      if (err instanceof StorageError) throw err;
      throw new StorageError(`SupabaseStorage delete failed: ${String(err)}`, key);
    }
  }

  async list(prefix: string): Promise<string[]> {
    const keys: string[] = [];
    let offset = 0;
    const limit = 100;

    try {
      while (true) {
        const res = await fetch(
          `${this.url}/storage/v1/object/list/${encodeURIComponent(this.bucket)}`,
          {
            method: "POST",
            headers: this.headers({ "Content-Type": "application/json" }),
            body: JSON.stringify({ prefix, limit, offset }),
          },
        );
        if (!res.ok) {
          const body = await res.text().catch(() => "");
          throw new StorageError(`SupabaseStorage list failed (${res.status}): ${body}`);
        }
        const page = (await res.json()) as Array<{ name: string }>;
        keys.push(...page.map((o) => o.name));
        if (page.length < limit) break;
        offset += limit;
      }
    } catch (err) {
      if (err instanceof StorageError) throw err;
      throw new StorageError(`SupabaseStorage list failed: ${String(err)}`);
    }

    return keys;
  }

  async signedUrl(key: string, ttlSeconds: number): Promise<string> {
    try {
      const res = await fetch(
        `${this.url}/storage/v1/object/sign/${encodeURIComponent(this.bucket)}/${key}`,
        {
          method: "POST",
          headers: this.headers({ "Content-Type": "application/json" }),
          body: JSON.stringify({ expiresIn: ttlSeconds }),
        },
      );
      if (!res.ok) {
        const body = await res.text().catch(() => "");
        throw new StorageError(
          `SupabaseStorage signedUrl failed (${res.status}): ${body}`,
          key,
        );
      }
      const data = (await res.json()) as { signedURL?: string };
      const signed = data.signedURL;
      if (!signed) {
        throw new StorageError("SupabaseStorage signedUrl: no signedURL in response", key);
      }
      return signed;
    } catch (err) {
      if (err instanceof StorageError) throw err;
      throw new StorageError(`SupabaseStorage signedUrl failed: ${String(err)}`, key);
    }
  }
}
