/**
 * Bot protection contract — verify a challenge token from a form submission.
 *
 * Mirrors `fiducial_adapters::bot_protection` in Rust for the contract and
 * `none` — but `turnstile` is real only here. Every product shape in this
 * repository that submits a form does so from TypeScript (a Next.js/SvelteKit
 * server action, or a Cloudflare Worker), so that is where verification
 * happens. See the Rust module's doc comment for the full reasoning.
 *
 * Designed against the narrowest interface shared by Turnstile, reCAPTCHA and
 * hCaptcha: all three are "the client solved a widget, hand me the token,
 * tell me if it was real."
 */

/** What the verification provider reported back. */
export interface VerifyOutcome {
  /** Whether the token was valid and unexpired. */
  success: boolean;
  /** Provider-reported timestamp of the challenge solve, ISO 8601. */
  challengeTs?: string;
  /** The hostname the widget was served from, as the provider saw it. */
  hostname?: string;
}

export class BotProtectionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "BotProtectionError";
  }
}

/** Bot / abuse challenge verification contract. */
export interface BotProtection {
  /**
   * Verify a client-solved challenge token.
   *
   * `remoteIp` is optional context some providers use to strengthen the
   * check; omitting it must never be treated as a failure.
   */
  verify(token: string, remoteIp?: string): Promise<VerifyOutcome>;
}

// ── None implementation ───────────────────────────────────────────────────────

/**
 * No-op bot protection — **fails open**: every token verifies as success.
 *
 * `botProtection = "none"` means the product has **no bot protection at
 * all**, not "protection pending." That is the correct default cost — a
 * product with no adversary yet should not be blocked by an unconfigured
 * challenge widget — but it must never be mistaken for "protected."
 */
export class NoneBotProtection implements BotProtection {
  // Accepts and ignores `env`, matching every other vendor class in this
  // contract, for the generated factory to call uniformly.
  constructor(_env?: unknown) {}

  async verify(_token: string, _remoteIp?: string): Promise<VerifyOutcome> {
    return { success: true };
  }
}

// ── Turnstile ────────────────────────────────────────────────────────────────

const TURNSTILE_VERIFY_URL =
  "https://challenges.cloudflare.com/turnstile/v0/siteverify";

interface TurnstileSiteverifyResponse {
  success: boolean;
  challenge_ts?: string;
  hostname?: string;
  ["error-codes"]?: string[];
}

/**
 * Cloudflare Turnstile — verifies a widget-solved token against
 * Cloudflare's `siteverify` endpoint over plain HTTPS.
 *
 * Unlike `D1Database`/`R2Storage`, this is **not** binding-reached — it is a
 * secret key (`env.TURNSTILE_SECRET_KEY`, set via `wrangler secret put`) and
 * a POST request, so it works from any TypeScript runtime with `fetch`: a
 * Worker, a Next.js server action, or plain Node.
 *
 * `fetchImpl` defaults to the global `fetch` and exists so tests can pass a
 * stub instead of monkeypatching `globalThis.fetch`.
 */
export class Turnstile implements BotProtection {
  private readonly secretKey: string;
  private readonly fetchImpl: typeof fetch;

  constructor(
    env: { TURNSTILE_SECRET_KEY?: string },
    fetchImpl: typeof fetch = fetch,
  ) {
    if (!env?.TURNSTILE_SECRET_KEY) {
      throw new BotProtectionError(
        "Turnstile: no `TURNSTILE_SECRET_KEY` on env — set it with " +
          "`wrangler secret put TURNSTILE_SECRET_KEY`",
      );
    }
    this.secretKey = env.TURNSTILE_SECRET_KEY;
    this.fetchImpl = fetchImpl;
  }

  async verify(token: string, remoteIp?: string): Promise<VerifyOutcome> {
    const body = new URLSearchParams({
      secret: this.secretKey,
      response: token,
    });
    if (remoteIp) body.set("remoteip", remoteIp);

    let res: Response;
    try {
      res = await this.fetchImpl(TURNSTILE_VERIFY_URL, {
        method: "POST",
        body,
      });
    } catch (err) {
      throw new BotProtectionError(`Turnstile request failed: ${String(err)}`);
    }

    if (!res.ok) {
      throw new BotProtectionError(
        `Turnstile siteverify returned HTTP ${res.status}`,
      );
    }

    const data = (await res.json()) as TurnstileSiteverifyResponse;
    return {
      success: data.success,
      challengeTs: data.challenge_ts,
      hostname: data.hostname,
    };
  }
}
