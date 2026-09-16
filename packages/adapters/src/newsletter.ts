/**
 * Newsletter contract — subscriber list management: subscribe, unsubscribe,
 * status.
 *
 * Mirrors `fiducial_adapters::newsletter` in Rust for the contract and
 * `none` — but `resend` is real only here. Every product shape in this
 * repository that renders a subscribe form does so from TypeScript (a
 * Next.js/SvelteKit server action, or a Cloudflare Worker). See the Rust
 * module's doc comment for the full reasoning.
 *
 * Designed against the intersection shared by Resend, Buttondown, Loops,
 * Listmonk and Mailchimp: subscribe an address to a list, mark it
 * unsubscribed (never delete — the opt-out record is a suppression entry),
 * and retrieve its current status.
 *
 * **This is not the same contract as `email`.** Resend itself makes the
 * split: `POST /emails` and `POST /audiences/{id}/contacts` are independent
 * surfaces, designed against different consumers, and Resend is currently
 * migrating the contacts surface from Audiences to a Global Contacts model
 * (see `ResendNewsletter`). Two contracts with independent rates of change
 * is what that is.
 */

/** Optional attributes carried by a subscriber. */
export interface SubscriberAttributes {
  firstName?: string;
  lastName?: string;
}

/** The canonical subscriber record returned by every method. */
export interface Subscription {
  /** Provider-assigned contact ID. Empty string when the adapter is `none`. */
  id: string;
  email: string;
  /** `true` while the address is actively subscribed; `false` after unsubscribe. */
  subscribed: boolean;
}

export class NewsletterError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "NewsletterError";
  }
}

/** Subscriber list management contract. */
export interface Newsletter {
  /**
   * Add `email` to the list, or re-subscribe it if it was previously
   * unsubscribed. Idempotent: calling this for an already-subscribed address
   * must not fail and must return the current `Subscription`.
   */
  subscribe(
    email: string,
    attrs?: SubscriberAttributes,
  ): Promise<Subscription>;

  /**
   * Mark `email` as unsubscribed. Does not delete the record — the opt-out
   * entry serves as a suppression list entry, which is the one thing it
   * exists to preserve.
   */
  unsubscribe(email: string): Promise<void>;

  /**
   * Return the current subscription record, or `null` if the address has
   * never been on the list.
   */
  status(email: string): Promise<Subscription | null>;
}

// ── None implementation ───────────────────────────────────────────────────────

/**
 * No-op newsletter — every call succeeds silently and no data is stored.
 *
 * `newsletter = "none"` discards subscriptions rather than collecting them.
 * Contrast with `NoneBotProtection`, which *fails open* and documents that
 * failure prominently — discarding a newsletter subscription is safe (no
 * address is leaked, no suppression list is bypassed), so there is no sharp
 * edge to label here. It matches `NoneQueue`'s posture.
 */
export class NoneNewsletter implements Newsletter {
  // Accepts and ignores `env`, matching every other vendor class in this
  // contract, for the generated factory to call uniformly.
  constructor(_env?: unknown) {}

  async subscribe(
    email: string,
    _attrs?: SubscriberAttributes,
  ): Promise<Subscription> {
    return { id: "", email, subscribed: true };
  }

  async unsubscribe(_email: string): Promise<void> {}

  async status(_email: string): Promise<Subscription | null> {
    return null;
  }
}

// ── Resend ────────────────────────────────────────────────────────────────────

const RESEND_CONTACTS_BASE = "https://api.resend.com/audiences";

/**
 * Narrow response shapes from Resend's contacts API.
 *
 * Resend is mid-migration from Audiences to a Global Contacts model
 * ("Segments"). The audience-scoped routes used here
 * (`/audiences/{id}/contacts`) are confirmed active as of 2026-09-16 via
 * context7. The `audience_id` constructor parameter makes the list identifier
 * a runtime configuration value, so the migration is a config change rather
 * than a code change. If a route 404s in a way that suggests it has been
 * retired, surface the error rather than guessing at a replacement path.
 */
interface ResendContactResponse {
  object: "contact";
  id: string;
}

interface ResendContactRecord {
  id: string;
  email: string;
  unsubscribed: boolean;
}

/**
 * Resend newsletter adapter — manages a subscriber list via Resend's
 * audience-scoped Contacts API.
 *
 * Unlike `D1Database`/`R2Storage`, this is **not** binding-reached — it is
 * two secrets (`env.RESEND_API_KEY` and `env.RESEND_AUDIENCE_ID`, set via
 * `wrangler secret put`) and plain HTTPS calls, so it works from any
 * TypeScript runtime with `fetch`.
 *
 * `fetchImpl` defaults to global `fetch` and exists so tests can pass a stub
 * instead of monkeypatching `globalThis.fetch`.
 */
export class ResendNewsletter implements Newsletter {
  private readonly apiKey: string;
  private readonly audienceId: string;
  private readonly fetchImpl: typeof fetch;

  constructor(
    env: { RESEND_API_KEY?: string; RESEND_AUDIENCE_ID?: string },
    fetchImpl: typeof fetch = fetch,
  ) {
    if (!env?.RESEND_API_KEY) {
      throw new NewsletterError(
        "ResendNewsletter: no `RESEND_API_KEY` on env — set it with " +
          "`wrangler secret put RESEND_API_KEY`",
      );
    }
    if (!env?.RESEND_AUDIENCE_ID) {
      throw new NewsletterError(
        "ResendNewsletter: no `RESEND_AUDIENCE_ID` on env — set it with " +
          "`wrangler secret put RESEND_AUDIENCE_ID`",
      );
    }
    this.apiKey = env.RESEND_API_KEY;
    this.audienceId = env.RESEND_AUDIENCE_ID;
    this.fetchImpl = fetchImpl;
  }

  async subscribe(
    email: string,
    attrs?: SubscriberAttributes,
  ): Promise<Subscription> {
    const url = `${RESEND_CONTACTS_BASE}/${this.audienceId}/contacts`;
    let res: Response;
    try {
      res = await this.fetchImpl(url, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          email,
          first_name: attrs?.firstName,
          last_name: attrs?.lastName,
          unsubscribed: false,
        }),
      });
    } catch (err) {
      throw new NewsletterError(
        `ResendNewsletter subscribe request failed: ${String(err)}`,
      );
    }

    if (res.status === 409 || res.status === 422) {
      // Contact already exists — fall back to a status read so the call
      // resolves to the current Subscription record (subscribe is idempotent).
      const existing = await this.status(email);
      if (existing) {
        // If the contact is currently unsubscribed, re-subscribe it.
        if (!existing.subscribed) {
          const resubscribed = await this.setUnsubscribed(email, false);
          // `null` means it vanished between the read and the write. Fall
          // through to the error path rather than claiming a subscription
          // that does not exist.
          if (resubscribed) return resubscribed;
        } else {
          return existing;
        }
      }
      // If status returns null after a conflict, something unexpected happened;
      // fall through to the error path below.
    }

    if (!res.ok) {
      throw new NewsletterError(
        `ResendNewsletter subscribe returned HTTP ${res.status}`,
      );
    }

    const data = (await res.json()) as ResendContactResponse;
    return { id: data.id, email, subscribed: true };
  }

  /**
   * Suppress an address.
   *
   * **A contact Resend does not have is success, not an error.** The caller
   * asked for "this person is not subscribed," and that state already holds;
   * throwing would turn a correctly-handled unsubscribe link into an error
   * page. Suppression is the one operation here that must never fail loudly —
   * it is the request a product is obliged to honour, and a caller that sees
   * an exception has no safe way to distinguish "already gone" from "not
   * applied."
   *
   * Every other failure — auth, rate limit, a 5xx — still throws, because
   * those genuinely did not apply the change.
   */
  async unsubscribe(email: string): Promise<void> {
    await this.setUnsubscribed(email, true);
  }

  async status(email: string): Promise<Subscription | null> {
    const url = `${RESEND_CONTACTS_BASE}/${this.audienceId}/contacts/${encodeURIComponent(email)}`;
    let res: Response;
    try {
      res = await this.fetchImpl(url, {
        headers: { Authorization: `Bearer ${this.apiKey}` },
      });
    } catch (err) {
      throw new NewsletterError(
        `ResendNewsletter status request failed: ${String(err)}`,
      );
    }

    if (res.status === 404) return null;

    if (!res.ok) {
      throw new NewsletterError(
        `ResendNewsletter status returned HTTP ${res.status}`,
      );
    }

    const data = (await res.json()) as ResendContactRecord;
    return {
      id: data.id,
      email: data.email,
      subscribed: !data.unsubscribed,
    };
  }

  /**
   * PATCH the contact's `unsubscribed` flag.
   *
   * Returns the updated record, or `null` when Resend has no such contact.
   * The 404 is returned rather than thrown because the two callers want
   * opposite things from it: `unsubscribe` treats a missing contact as the
   * desired end state, and `subscribe`'s re-subscribe path treats it as the
   * race it is. Deciding that here would be wrong for one of them.
   */
  private async setUnsubscribed(
    email: string,
    unsubscribed: boolean,
  ): Promise<Subscription | null> {
    const url = `${RESEND_CONTACTS_BASE}/${this.audienceId}/contacts/${encodeURIComponent(email)}`;
    let res: Response;
    try {
      res = await this.fetchImpl(url, {
        method: "PATCH",
        headers: {
          Authorization: `Bearer ${this.apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ unsubscribed }),
      });
    } catch (err) {
      throw new NewsletterError(
        `ResendNewsletter update request failed: ${String(err)}`,
      );
    }

    if (res.status === 404) return null;

    if (!res.ok) {
      throw new NewsletterError(
        `ResendNewsletter update returned HTTP ${res.status}`,
      );
    }

    const data = (await res.json()) as ResendContactRecord;
    return {
      id: data.id,
      email: data.email,
      subscribed: !data.unsubscribed,
    };
  }
}
