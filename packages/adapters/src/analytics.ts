/**
 * Analytics contract — one contract, many providers, cookieless by default.
 *
 * Site code calls only the `Analytics` interface. `fid derive` generates a
 * fan-out factory (`apps/web/src/generated/analytics.ts`) from `[analytics]`
 * in `fiducial.toml`; adding or removing a provider is a config edit followed
 * by `fid derive`, never a site file change.
 *
 * Spec: `docs/specs/2026-11-23-analytics-is-a-capability-consent-is-opt-in.md`
 * D16–D19.
 */

export interface PageviewContext {
  path: string;
  utm_source?: string;
  utm_medium?: string;
  utm_campaign?: string;
  referrer?: string;
}

export type AnalyticsEventName =
  | "page_view"
  | "cta_click"
  | "add_to_cart"
  | "checkout_start"
  | "quote_request"
  | "order_submitted";

export interface AnalyticsEvent {
  name: AnalyticsEventName;
  path: string;
  utm_source?: string;
  utm_medium?: string;
  utm_campaign?: string;
  referrer?: string;
  [key: string]: string | number | boolean | undefined;
}

export interface Analytics {
  pageview(ctx: PageviewContext): Promise<void>;
  track(event: AnalyticsEvent): Promise<void>;
}

export class NoneAnalytics implements Analytics {
  async pageview(_ctx: PageviewContext): Promise<void> {}
  async track(_event: AnalyticsEvent): Promise<void> {}
}

export class FirstPartyAnalytics implements Analytics {
  private readonly origin: string;

  constructor(_env?: unknown, origin?: string) {
    this.origin = origin ?? (typeof window !== "undefined" ? window.location.origin : "");
  }

  async pageview(ctx: PageviewContext): Promise<void> {
    await this.track({ name: "page_view", ...ctx });
  }

  async track(event: AnalyticsEvent): Promise<void> {
    try {
      await fetch(`${this.origin}/api/e`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(event),
      });
    } catch {
      // best-effort
    }
  }
}

export class CloudflareWebAnalytics implements Analytics {
  private readonly token: string;

  constructor(env?: { CF_BEACON_TOKEN?: string }) {
    this.token = env?.CF_BEACON_TOKEN ?? "";
  }

  async pageview(_ctx: PageviewContext): Promise<void> {
    // The Cloudflare beacon is loaded as a <script> tag in the layout;
    // pageview calls are handled by the beacon itself.
  }

  async track(_event: AnalyticsEvent): Promise<void> {
    // Custom event tracking through the Cloudflare Web Analytics JS SDK is
    // not yet implemented. The beacon fires on every page load automatically.
  }
}

export class PostHogAnalytics implements Analytics {
  private readonly apiKey: string;

  constructor(env?: { POSTHOG_API_KEY?: string }) {
    this.apiKey = env?.POSTHOG_API_KEY ?? "";
  }

  async pageview(ctx: PageviewContext): Promise<void> {
    await this.send("$pageview", { $current_url: ctx.path });
  }

  async track(event: AnalyticsEvent): Promise<void> {
    await this.send(event.name, event);
  }

  private async send(event: string, properties: Record<string, unknown>): Promise<void> {
    if (!this.apiKey) return;
    try {
      await fetch("https://eu.i.posthog.com/capture/", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          api_key: this.apiKey,
          event,
          properties: {
            ...properties,
            $lib: "fiducial-analytics",
            $process_person_profile: false,
          },
          timestamp: new Date().toISOString(),
        }),
      });
    } catch {
      // best-effort
    }
  }
}

export class MetaPixelCapi implements Analytics {
  private readonly pixelId: string;
  private readonly accessToken: string;

  constructor(env?: { META_PIXEL_ID?: string; META_ACCESS_TOKEN?: string }) {
    this.pixelId = env?.META_PIXEL_ID ?? "";
    this.accessToken = env?.META_ACCESS_TOKEN ?? "";
  }

  async pageview(ctx: PageviewContext): Promise<void> {
    await this.track({ name: "page_view", ...ctx });
  }

  async track(event: AnalyticsEvent): Promise<void> {
    if (!this.pixelId || !this.accessToken) return;
    try {
      await fetch(
        `https://graph.facebook.com/v18.0/${this.pixelId}/events?access_token=${this.accessToken}`,
        {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({
            data: [
              {
                event_name: event.name,
                event_time: Math.floor(Date.now() / 1000),
                event_source_url: event.path,
                action_source: "website",
              },
            ],
          }),
        },
      );
    } catch {
      // best-effort
    }
  }
}
