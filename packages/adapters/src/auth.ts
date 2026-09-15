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

/**
 * The signed-in user.
 *
 * `emailVerified` and `createdAt` are nullable because a session verified
 * from a bearer JWT alone genuinely does not carry them — the claims have a
 * subject and an email, not a confirmation timestamp. `null` there means
 * "this delivery model cannot tell you," which a caller can act on; a
 * fabricated `false`/`""` would be a claim about the user that is not true.
 */
export interface AuthUser {
  id: string;
  email: string | null;
  emailVerified: boolean | null;
  /** ISO 8601, or `null` when unknown from this session's material. */
  createdAt: string | null;
}

/** A live session: the tokens plus the user they belong to. */
export interface AuthSession {
  accessToken: string;
  /**
   * `null` under bearer delivery: the client holds its own refresh token and
   * the server never sees one.
   */
  refreshToken: string | null;
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
  // Accepts and ignores `env` and `ctx` so `createAuth(env, ctx)` can
  // construct any vendor class uniformly — see the generated factory in
  // `fid-adapters`'s `derive.rs` executor.
  constructor(
    _env?: unknown,
    _ctx?: AuthSessionContext,
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
 * In-memory store — the SDK's scratch space when nothing should persist
 * between requests. Used by `BearerSessionContext`, where the calling client
 * owns the session and re-sends its access token itself.
 */
export class MemoryKeyValueStore implements AuthKeyValueStore {
  private readonly data = new Map<string, string>();

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

// ── Session context: how this request carries its session ────────────────────

/**
 * How one request carries its session — the second argument to
 * `createAuth(env, ctx)`.
 *
 * Two things, because the two delivery models genuinely need two different
 * things and an earlier design that tried to express both as "a key/value
 * store" shipped broken: bearer delivery had its token written under a key
 * the SDK never reads (`sb-access-token`, where the SDK reads a JSON session
 * from its own `storageKey`), so `getSession()` silently returned null for
 * every bearer request. Naming the token separately is what makes the bearer
 * path exist at all.
 */
export interface AuthSessionContext {
  /** Where the vendor SDK persists session and PKCE state. */
  readonly storage: AuthKeyValueStore;
  /**
   * The access token this request supplied directly, for bearer delivery.
   * `null`/absent means the session lives in `storage` instead (cookies).
   */
  readonly bearerToken?: string | null;
}

/**
 * Cookie-delivered sessions — server-rendered apps (web-next, web-svelte).
 *
 * Supports the full OAuth redirect flow: the PKCE verifier written during
 * `signInWithOAuth` survives to the callback request because it is in a
 * cookie.
 */
export class CookieSessionContext implements AuthSessionContext {
  readonly storage: AuthKeyValueStore;

  constructor(
    cookies: {
      get(name: string): string | undefined;
      set(name: string, value: string, options?: Record<string, unknown>): void;
      delete(name: string): void;
    },
    cookieOptions?: Record<string, unknown>,
  ) {
    this.storage = new CookieKeyValueStore(cookies, cookieOptions);
  }
}

/**
 * Bearer-delivered sessions — a Tauri desktop app, a future mobile client,
 * or any API caller that holds its own token and sends it per request.
 *
 * Nothing persists server-side: `storage` is in-memory for the SDK's own
 * scratch use, and the session itself is whatever `bearerToken` verifies to.
 *
 * **Does not support the OAuth redirect flow.** PKCE state must survive
 * between the redirect-out request and the callback request, and without a
 * cookie there is nowhere server-side to keep it. `signInWithOAuth` and
 * `exchangeCodeForSession` throw a clear `AuthError` under this context
 * rather than silently losing the verifier. A native client performs OAuth
 * directly against the vendor and sends the resulting access token here
 * afterward — this server's job there is verification, not orchestrating a
 * redirect it has no page to render.
 */
export class BearerSessionContext implements AuthSessionContext {
  readonly storage: AuthKeyValueStore = new MemoryKeyValueStore();

  constructor(readonly bearerToken: string | null = null) {}

  /** Build from an incoming `Authorization: Bearer <token>` header value. */
  static fromAuthorizationHeader(header: string | null): BearerSessionContext {
    return new BearerSessionContext(
      header?.match(/^Bearer\s+(.+)$/i)?.[1] ?? null,
    );
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
  /**
   * Verifies a JWT and returns its claims — locally against the project's
   * JWKS when the signing key is asymmetric and WebCrypto is available,
   * falling back to a network `getUser()` validation otherwise. Either way
   * the signature is checked, which is the whole reason `getSession` below
   * is never trusted on its own.
   */
  getClaims(jwt?: string): Promise<{
    data: { claims: SupabaseJwtClaims } | null;
    error: SupabaseAuthApiError | null;
  }>;
}

/** The subset of Supabase's JWT claims this adapter reads. */
interface SupabaseJwtClaims {
  sub: string;
  email?: string;
  exp?: number;
  role?: string;
  is_anonymous?: boolean;
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
) => SupabaseClientLike | Promise<SupabaseClientLike>;

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
 * Session and PKCE-verifier persistence go through the session context's
 * `AuthKeyValueStore` (`storage` in the client's own `auth` options) rather
 * than the SDK's default `localStorage`/`AsyncStorage`, which do not exist
 * in a Worker or a Next.js server action. This is the SDK's own documented
 * extension point, not a workaround.
 *
 * Construct one `SupabaseAuth` **per request**, with a context scoped to that
 * request (`new CookieSessionContext(cookieJar)`, or
 * `BearerSessionContext.fromAuthorizationHeader(header)`) — the same reason a
 * fresh `createServerClient` is constructed per request in `@supabase/ssr`'s
 * own documented pattern. This is why `auth` is not part of
 * `AdapterSet`/`createAdapters(env)`: every other slot there is env-scoped,
 * and this one is request-scoped.
 *
 * ## Every session this returns has had its signature verified
 *
 * `getSession()` never trusts the SDK's own `getSession()` on its own. That
 * call reads the session out of storage and checks only `expires_at` — on a
 * server, storage is a *client-supplied cookie*, so a forged cookie would
 * otherwise yield a "valid" session with any `user.id` the caller liked.
 * Every path here runs the access token through `getClaims()`, which
 * verifies the signature locally against the project's JWKS when the signing
 * key is asymmetric, and falls back to a network `getUser()` validation
 * otherwise.
 */
export class SupabaseAuth implements Auth {
  private readonly clientPromise: Promise<SupabaseClientLike>;
  private readonly ctx: AuthSessionContext;

  constructor(
    env: { SUPABASE_URL?: string; SUPABASE_ANON_KEY?: string },
    ctx: AuthSessionContext,
    clientFactory: SupabaseClientFactory = defaultClientFactory,
  ) {
    if (!env?.SUPABASE_URL || !env?.SUPABASE_ANON_KEY) {
      throw new AuthError(
        "SupabaseAuth: env needs SUPABASE_URL and SUPABASE_ANON_KEY",
      );
    }
    if (!ctx?.storage) {
      throw new AuthError(
        "SupabaseAuth: needs an AuthSessionContext — CookieSessionContext for " +
          "server-rendered apps, BearerSessionContext for a token-bearing client",
      );
    }
    this.ctx = ctx;
    this.clientPromise = Promise.resolve(
      clientFactory(env.SUPABASE_URL, env.SUPABASE_ANON_KEY, ctx.storage),
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

  /** OAuth needs somewhere to keep PKCE state across the redirect. */
  private requireRedirectCapableContext(method: string): void {
    if (this.ctx.bearerToken != null || !(this.ctx instanceof CookieSessionContext)) {
      throw new AuthError(
        `${method} needs a cookie-backed session context to persist PKCE ` +
          "state across the redirect — see BearerSessionContext's doc comment",
      );
    }
  }

  async signInWithOAuth(
    provider: string,
    redirectTo: string,
  ): Promise<{ url: string }> {
    this.requireRedirectCapableContext("signInWithOAuth");
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
    this.requireRedirectCapableContext("exchangeCodeForSession");
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

  /**
   * The current session, **signature-verified**, or `null`.
   *
   * Bearer delivery verifies the token the request supplied. Cookie delivery
   * reads the stored session for its access token and user, then verifies
   * that token before returning anything — see this class's doc comment for
   * why the SDK's own `getSession()` is never trusted alone.
   */
  async getSession(): Promise<AuthSession | null> {
    const client = await this.client();

    if (this.ctx.bearerToken != null) {
      const claims = await this.verifiedClaims(client, this.ctx.bearerToken);
      if (!claims) return null;
      return {
        accessToken: this.ctx.bearerToken,
        // A bearer client holds its own refresh token; the server never sees
        // one, and inventing an empty string here would read as "there is a
        // refresh token and it is blank."
        refreshToken: null,
        expiresAt: claims.exp ?? 0,
        user: {
          id: claims.sub,
          email: claims.email ?? null,
          // Not carried in the JWT: a caller that needs confirmation state
          // reads the user record, rather than this claiming to know.
          emailVerified: null,
          createdAt: null,
        },
      };
    }

    const { data, error } = await client.getSession();
    throwIfError(error);
    if (!data.session) return null;

    const claims = await this.verifiedClaims(client, data.session.access_token);
    if (!claims) return null;
    // The stored user is only trusted once the token carrying it verified,
    // and only when the token's subject is the user the cookie claims.
    if (claims.sub !== data.session.user.id) return null;
    return toAuthSession(data.session);
  }

  /** Verified claims for `token`, or `null` when it does not verify. */
  private async verifiedClaims(
    client: SupabaseAuthClient,
    token: string,
  ): Promise<SupabaseJwtClaims | null> {
    const { data, error } = await client.getClaims(token);
    if (error || !data?.claims?.sub) return null;
    return data.claims;
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
