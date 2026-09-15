/**
 * Auth contract — users and authentication: sign-up, sign-in, sessions.
 *
 * Mirrors `fiducial_adapters::auth` in Rust for the contract and `none`.
 * `supabase` is real only here — see the Rust module's doc comment for why.
 *
 * Designed against the narrowest interface shared by Supabase Auth, Clerk
 * and Auth.js: email/password sign-up and sign-in, OAuth via redirect,
 * sign-out, password reset, and reading the current session.
 */

/** The signed-in principal. */
export interface AuthUser {
  id: string;
  email: string | null;
  emailVerified: boolean;
  /** ISO 8601. */
  createdAt: string;
}

/** A live session: the tokens plus the user they belong to. */
export interface AuthSession {
  accessToken: string;
  refreshToken: string;
  /** Unix seconds. */
  expiresAt: number;
  user: AuthUser;
}

export class AuthError extends Error {
  constructor(
    message: string,
    public readonly code?: string,
  ) {
    super(message);
    this.name = "AuthError";
  }
}

/** Users and authentication contract. */
export interface Auth {
  signUp(email: string, password: string): Promise<AuthSession>;
  signIn(email: string, password: string): Promise<AuthSession>;

  /**
   * Start an OAuth sign-in: returns the URL to redirect the user to.
   *
   * Meaningful only under cookie-delivered sessions (`CookieKeyValueStore`)
   * — the redirect/callback round trip needs somewhere to persist PKCE
   * state between the two requests, and a bearer client has nowhere
   * server-side to put it. A native/desktop client performs OAuth directly
   * against the vendor itself and sends the resulting session's access
   * token to this API afterward; this server's job there is verification
   * (`getSession`), not orchestrating the redirect. See `SupabaseAuth`'s
   * doc comment.
   */
  signInWithOAuth(
    provider: string,
    redirectTo: string,
  ): Promise<{ url: string }>;

  /** Complete an OAuth sign-in from the callback's `code` query param. */
  exchangeCodeForSession(code: string): Promise<AuthSession>;

  signOut(): Promise<void>;

  /** The current session, if any — verified and refreshed as needed. */
  getSession(): Promise<AuthSession | null>;

  /** Send a password-reset email; `redirectTo` is where the link lands. */
  resetPasswordForEmail(email: string, redirectTo: string): Promise<void>;

  /** Set a new password for the currently authenticated user. */
  updatePassword(newPassword: string): Promise<void>;
}

// ── None implementation ───────────────────────────────────────────────────────

const NOT_CONFIGURED = 'auth = "none" — no auth vendor configured';

/**
 * No-op auth — every write throws, `getSession` always reports signed-out.
 *
 * Unlike most `None*` types, this does not silently succeed: a product that
 * has not chosen an auth vendor should fail loudly on a sign-in attempt, not
 * fabricate a session. `auth = "none"` means "no auth wired up," not "auth
 * that always works."
 */
export class NoneAuth implements Auth {
  // Accepts and ignores `env` and `store` so `createAuth(env, store)` can
  // construct any vendor class uniformly — see the generated factory in
  // `fid-adapters`'s `derive.rs` executor.
  constructor(
    _env?: unknown,
    _store?: AuthKeyValueStore,
  ) {}

  async signUp(): Promise<AuthSession> {
    throw new AuthError(NOT_CONFIGURED);
  }

  async signIn(): Promise<AuthSession> {
    throw new AuthError(NOT_CONFIGURED);
  }

  async signInWithOAuth(): Promise<{ url: string }> {
    throw new AuthError(NOT_CONFIGURED);
  }

  async exchangeCodeForSession(): Promise<AuthSession> {
    throw new AuthError(NOT_CONFIGURED);
  }

  async signOut(): Promise<void> {}

  async getSession(): Promise<AuthSession | null> {
    return null;
  }

  async resetPasswordForEmail(): Promise<void> {
    throw new AuthError(NOT_CONFIGURED);
  }

  async updatePassword(): Promise<void> {
    throw new AuthError(NOT_CONFIGURED);
  }
}

// ── Session key/value storage ───────────────────────────────────────────────

/**
 * The extension point `@supabase/supabase-js` itself defines
 * (`SupportedStorage`) for persisting session and PKCE state — a plain
 * async key/value store. Implementing this, rather than inventing a
 * Fiducial-specific shape, is the "narrowest shared interface" rule applied
 * to storage instead of to the auth operations themselves: it is the same
 * extension point `@supabase/ssr`'s own cookie adapter fills, just without
 * that package's chunked-cookie serialization format as a dependency.
 */
export interface AuthKeyValueStore {
  getItem(key: string): Promise<string | null> | string | null;
  setItem(key: string, value: string): Promise<void> | void;
  removeItem(key: string): Promise<void> | void;
}

/**
 * Cookie-backed store for server-rendered apps (web-next, web-svelte).
 *
 * One JSON-serialized cookie per key. Sessions and PKCE verifiers are small
 * enough for one cookie each (unlike `@supabase/ssr`'s own multi-chunk
 * format, built for a larger combined payload) — if a future vendor's
 * session material grows past a single cookie, chunking becomes this
 * class's problem to solve, not the contract's.
 *
 * `read`/`write`/`remove` are framework-agnostic on purpose: pass whatever
 * your server framework gives you for reading/writing response cookies.
 */
export class CookieKeyValueStore implements AuthKeyValueStore {
  constructor(
    private readonly cookies: {
      get(name: string): string | undefined;
      set(name: string, value: string, options?: Record<string, unknown>): void;
      delete(name: string): void;
    },
    private readonly cookieOptions: Record<string, unknown> = {
      httpOnly: true,
      secure: true,
      sameSite: "lax",
      path: "/",
    },
  ) {}

  getItem(key: string): string | null {
    return this.cookies.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.cookies.set(key, value, this.cookieOptions);
  }

  removeItem(key: string): void {
    this.cookies.delete(key);
  }
}

/**
 * In-memory store for bearer/native delivery (Tauri, a future mobile
 * client): the server does not persist anything between requests — the
 * calling client owns the session and sends its access token back itself.
 *
 * **Does not support the OAuth redirect flow.** PKCE state needs to survive
 * between the redirect-out request and the callback request; without a
 * cookie there is nowhere server-side to keep it, and this store's lifetime
 * is one request. `SupabaseAuth.signInWithOAuth`/`exchangeCodeForSession`
 * throw a clear `AuthError` under this store rather than silently losing
 * the verifier. A native client performs OAuth directly against Supabase
 * itself instead — see `Auth.signInWithOAuth`'s doc comment.
 */
export class BearerKeyValueStore implements AuthKeyValueStore {
  private readonly data = new Map<string, string>();

  /** Seed the store with an incoming `Authorization: Bearer <token>` value. */
  static fromAuthorizationHeader(header: string | null): BearerKeyValueStore {
    const store = new BearerKeyValueStore();
    const token = header?.match(/^Bearer\s+(.+)$/i)?.[1];
    if (token) {
      // Matches the key `@supabase/supabase-js` itself uses for the access
      // token half of a session under its default storage key prefix.
      store.data.set("sb-access-token", token);
    }
    return store;
  }

  getItem(key: string): string | null {
    return this.data.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.data.set(key, value);
  }

  removeItem(key: string): void {
    this.data.delete(key);
  }
}

// ── Supabase ─────────────────────────────────────────────────────────────────

/** The minimal shape of `@supabase/supabase-js`'s client this file calls. */
interface SupabaseAuthClient {
  signUp(credentials: {
    email: string;
    password: string;
  }): Promise<SupabaseAuthResponse>;
  signInWithPassword(credentials: {
    email: string;
    password: string;
  }): Promise<SupabaseAuthResponse>;
  signInWithOAuth(options: {
    provider: string;
    options: { redirectTo: string };
  }): Promise<{ data: { url: string | null }; error: SupabaseAuthApiError | null }>;
  exchangeCodeForSession(code: string): Promise<SupabaseAuthResponse>;
  signOut(): Promise<{ error: SupabaseAuthApiError | null }>;
  getSession(): Promise<{
    data: { session: SupabaseSession | null };
    error: SupabaseAuthApiError | null;
  }>;
  resetPasswordForEmail(
    email: string,
    options: { redirectTo: string },
  ): Promise<{ error: SupabaseAuthApiError | null }>;
  updateUser(attrs: {
    password: string;
  }): Promise<{ error: SupabaseAuthApiError | null }>;
}

interface SupabaseClientLike {
  auth: SupabaseAuthClient;
}

interface SupabaseAuthApiError {
  message: string;
  code?: string;
  status?: number;
}

interface SupabaseUser {
  id: string;
  email?: string;
  email_confirmed_at?: string | null;
  created_at: string;
}

interface SupabaseSession {
  access_token: string;
  refresh_token: string;
  expires_at?: number;
  user: SupabaseUser;
}

interface SupabaseAuthResponse {
  data: { session: SupabaseSession | null; user: SupabaseUser | null };
  error: SupabaseAuthApiError | null;
}

function toAuthUser(u: SupabaseUser): AuthUser {
  return {
    id: u.id,
    email: u.email ?? null,
    emailVerified: Boolean(u.email_confirmed_at),
    createdAt: u.created_at,
  };
}

function toAuthSession(s: SupabaseSession): AuthSession {
  return {
    accessToken: s.access_token,
    refreshToken: s.refresh_token,
    expiresAt: s.expires_at ?? 0,
    user: toAuthUser(s.user),
  };
}

function throwIfError(error: SupabaseAuthApiError | null): void {
  if (!error) return;
  if (error.status === 400 || error.code === "invalid_credentials") {
    throw new AuthError("invalid credentials", error.code);
  }
  if (error.status === 429) {
    throw new AuthError("rate limited", error.code);
  }
  throw new AuthError(error.message, error.code);
}

/**
 * A factory for the underlying `@supabase/supabase-js` client, injected so
 * tests can supply a fake instead of hitting the network. Defaults to the
 * real `createClient` from `@supabase/supabase-js`.
 */
export type SupabaseClientFactory = (
  url: string,
  anonKey: string,
  store: AuthKeyValueStore,
) => SupabaseClientLike;

async function defaultClientFactory(
  url: string,
  anonKey: string,
  store: AuthKeyValueStore,
): Promise<SupabaseClientLike> {
  const { createClient } = await import("@supabase/supabase-js");
  return createClient(url, anonKey, {
    auth: {
      storage: store,
      persistSession: true,
      autoRefreshToken: false,
      detectSessionInUrl: false,
      flowType: "pkce",
    },
  }) as unknown as SupabaseClientLike;
}

/**
 * Supabase Auth — real `Auth` implementation over `@supabase/supabase-js`.
 *
 * Session and PKCE-verifier persistence go through the injected
 * `AuthKeyValueStore` (`storage` in the client's own `auth` options) rather
 * than the SDK's default `localStorage`/`AsyncStorage`, which do not exist
 * in a Worker or a Next.js server action. This is the SDK's own documented
 * extension point, not a workaround.
 *
 * Construct one `SupabaseAuth` **per request**, with a store scoped to that
 * request (`CookieKeyValueStore` wrapping that request's cookie jar, or
 * `BearerKeyValueStore.fromAuthorizationHeader` for that request's header) —
 * the same reason a fresh `createServerClient` is constructed per request in
 * `@supabase/ssr`'s own documented pattern. This is why `auth` is not part
 * of `AdapterSet`/`createAdapters(env)`: every other slot there is
 * env-scoped, and this one is request-scoped.
 */
export class SupabaseAuth implements Auth {
  private readonly clientPromise: Promise<SupabaseClientLike>;

  constructor(
    env: { SUPABASE_URL?: string; SUPABASE_ANON_KEY?: string },
    private readonly store: AuthKeyValueStore,
    clientFactory: (
      url: string,
      anonKey: string,
      store: AuthKeyValueStore,
    ) => SupabaseClientLike | Promise<SupabaseClientLike> = defaultClientFactory,
  ) {
    if (!env?.SUPABASE_URL || !env?.SUPABASE_ANON_KEY) {
      throw new AuthError(
        "SupabaseAuth: env needs SUPABASE_URL and SUPABASE_ANON_KEY",
      );
    }
    this.clientPromise = Promise.resolve(
      clientFactory(env.SUPABASE_URL, env.SUPABASE_ANON_KEY, store),
    );
  }

  private async client(): Promise<SupabaseAuthClient> {
    return (await this.clientPromise).auth;
  }

  async signUp(email: string, password: string): Promise<AuthSession> {
    const { data, error } = await (await this.client()).signUp({ email, password });
    throwIfError(error);
    if (!data.session) {
      throw new AuthError(
        "sign-up succeeded but requires email confirmation before a session exists",
      );
    }
    return toAuthSession(data.session);
  }

  async signIn(email: string, password: string): Promise<AuthSession> {
    const { data, error } = await (
      await this.client()
    ).signInWithPassword({ email, password });
    throwIfError(error);
    if (!data.session) {
      throw new AuthError("sign-in returned no session");
    }
    return toAuthSession(data.session);
  }

  async signInWithOAuth(
    provider: string,
    redirectTo: string,
  ): Promise<{ url: string }> {
    if (this.store instanceof BearerKeyValueStore) {
      throw new AuthError(
        "signInWithOAuth needs a cookie-backed store to persist PKCE state " +
          "across the redirect — see BearerKeyValueStore's doc comment",
      );
    }
    const { data, error } = await (
      await this.client()
    ).signInWithOAuth({ provider, options: { redirectTo } });
    throwIfError(error);
    if (!data.url) {
      throw new AuthError("provider returned no redirect URL");
    }
    return { url: data.url };
  }

  async exchangeCodeForSession(code: string): Promise<AuthSession> {
    if (this.store instanceof BearerKeyValueStore) {
      throw new AuthError(
        "exchangeCodeForSession needs a cookie-backed store to read the " +
          "PKCE verifier saved by signInWithOAuth — see BearerKeyValueStore's doc comment",
      );
    }
    const { data, error } = await (await this.client()).exchangeCodeForSession(code);
    throwIfError(error);
    if (!data.session) {
      throw new AuthError("code exchange returned no session");
    }
    return toAuthSession(data.session);
  }

  async signOut(): Promise<void> {
    const { error } = await (await this.client()).signOut();
    throwIfError(error);
  }

  async getSession(): Promise<AuthSession | null> {
    const { data, error } = await (await this.client()).getSession();
    throwIfError(error);
    return data.session ? toAuthSession(data.session) : null;
  }

  async resetPasswordForEmail(email: string, redirectTo: string): Promise<void> {
    const { error } = await (
      await this.client()
    ).resetPasswordForEmail(email, { redirectTo });
    throwIfError(error);
  }

  async updatePassword(newPassword: string): Promise<void> {
    const { error } = await (await this.client()).updateUser({
      password: newPassword,
    });
    throwIfError(error);
  }
}
