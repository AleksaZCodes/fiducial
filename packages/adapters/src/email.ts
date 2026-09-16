/**
 * Email contract — transactional email: send, template, domain verification.
 *
 * Mirrors `fiducial_adapters::email` in Rust. Designed against Resend, SES,
 * and Cloudflare Email Routing.
 *
 * Unlike `D1Database`/`R2Storage`, `ResendEmail` is **not** binding-reached —
 * it is a secret key (`env.RESEND_API_KEY`, set via `wrangler secret put`)
 * and a POST request over plain HTTPS, so it works from any TypeScript runtime
 * with `fetch`. Same boundary as `Turnstile`.
 */

/** An outbound email message. */
export interface Message {
  /** `"Name <addr>"` or bare `"addr"`. */
  from: string;
  /** One or more recipient addresses. */
  to: string[];
  subject: string;
  /** HTML body (required). */
  html: string;
  /** Optional plain-text alternative. */
  text?: string;
  /** Optional reply-to address. */
  replyTo?: string;
}

export class EmailError extends Error {
  constructor(
    message: string,
    public readonly code?: string,
  ) {
    super(message);
    this.name = "EmailError";
  }
}

/** Transactional email contract. */
export interface Email {
  /**
   * Send a single transactional message.
   *
   * Returns the provider-assigned message ID on success.
   */
  send(message: Message): Promise<string>;
}

// ── None implementation ───────────────────────────────────────────────────────

/**
 * No-op email sender — all sends succeed silently, returning an empty ID.
 *
 * Switching to Resend in production is a config change, not a refactor.
 */
export class NoneEmail implements Email {
  // Accepts and ignores `env` so every vendor class in this contract shares
  // one constructor shape for the generated factory to call uniformly.
  constructor(_env?: unknown) {}

  async send(_message: Message): Promise<string> {
    return "";
  }
}

// ── Resend ────────────────────────────────────────────────────────────────────

const RESEND_SEND_URL = "https://api.resend.com/emails";

interface ResendSendResponse {
  id: string;
}

/**
 * Resend — sends transactional email via `POST https://api.resend.com/emails`.
 *
 * Secret-reached: `env.RESEND_API_KEY`, set with `wrangler secret put`.
 * Constructor throws `EmailError` when the key is absent, with the
 * `wrangler secret put` command in the message — a misconfigured product fails
 * at construction rather than at the first send.
 *
 * `fetchImpl` defaults to global `fetch` and exists so tests can pass a stub
 * instead of monkeypatching `globalThis.fetch`.
 *
 * Resend's documented default rate limit is 10 req/s per team. HTTP 429
 * is mapped onto `EmailError` because the Rust contract already carries a
 * `RateLimited` variant and this is a real path, not a defensive one.
 */
export class ResendEmail implements Email {
  private readonly apiKey: string;
  private readonly fetchImpl: typeof fetch;

  constructor(
    env: { RESEND_API_KEY?: string },
    fetchImpl: typeof fetch = fetch,
  ) {
    if (!env?.RESEND_API_KEY) {
      throw new EmailError(
        "ResendEmail: no `RESEND_API_KEY` on env — set it with " +
          "`wrangler secret put RESEND_API_KEY`",
      );
    }
    this.apiKey = env.RESEND_API_KEY;
    this.fetchImpl = fetchImpl;
  }

  async send(message: Message): Promise<string> {
    let res: Response;
    try {
      res = await this.fetchImpl(RESEND_SEND_URL, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          from: message.from,
          to: message.to,
          subject: message.subject,
          html: message.html,
          text: message.text,
          reply_to: message.replyTo,
        }),
      });
    } catch (err) {
      throw new EmailError(`ResendEmail request failed: ${String(err)}`);
    }

    if (res.status === 429) {
      throw new EmailError("ResendEmail: rate limited (HTTP 429)", "rate_limited");
    }

    if (!res.ok) {
      throw new EmailError(
        `ResendEmail returned HTTP ${res.status}`,
        String(res.status),
      );
    }

    const data = (await res.json()) as ResendSendResponse;
    return data.id;
  }
}
