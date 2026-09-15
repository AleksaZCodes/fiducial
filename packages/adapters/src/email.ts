/**
 * Email contract — transactional email: send, template, domain verification.
 *
 * Mirrors `fiducial_adapters::email` in Rust. Designed against Resend, SES,
 * and Cloudflare Email Routing.
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
  async send(_message: Message): Promise<string> {
    return "";
  }
}
