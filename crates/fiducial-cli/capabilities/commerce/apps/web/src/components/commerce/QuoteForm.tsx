"use client";

import { type FormEvent, useCallback, useEffect, useRef, useState } from "react";
import type { CommerceLeadSource } from "@/generated/commerce";

const UTM_KEY = "fid-utm";

interface UtmCapture {
  path?: string;
  utm_source?: string;
  utm_medium?: string;
  utm_campaign?: string;
  referrer?: string;
}

function readUtm(): UtmCapture {
  try {
    const raw = window.sessionStorage.getItem(UTM_KEY);
    if (raw) return JSON.parse(raw) as UtmCapture;
  } catch {}
  return {};
}

interface QuoteFormProps {
  productId?: string;
  productName?: string;
  leadSources: readonly CommerceLeadSource[];
  turnstileSiteKey: string;
}

type Status = "idle" | "submitting" | "success" | "error";

export default function QuoteForm({
  productId,
  productName,
  leadSources,
  turnstileSiteKey,
}: QuoteFormProps) {
  const [status, setStatus] = useState<Status>("idle");
  const [token, setToken] = useState<string | null>(null);
  const widgetRef = useRef<HTMLDivElement>(null);
  const widgetIdRef = useRef<string | number | undefined>(undefined);

  useEffect(() => {
    if (!turnstileSiteKey) return;
    if (document.querySelector('script[src*="turnstile"]')) return;
    const s = document.createElement("script");
    s.src = "https://challenges.cloudflare.com/turnstile/v0/api.js";
    s.defer = true;
    document.head.appendChild(s);
  }, [turnstileSiteKey]);

  useEffect(() => {
    if (!turnstileSiteKey || !widgetRef.current) return;
    const w = window as unknown as { turnstile?: { render: (el: HTMLElement, opts: unknown) => string | number; remove: (id: string | number) => void } };
    const tryRender = () => {
      if (!w.turnstile || !widgetRef.current) return false;
      widgetIdRef.current = w.turnstile.render(widgetRef.current, {
        sitekey: turnstileSiteKey,
        callback: (t: string) => setToken(t),
        "expired-callback": () => setToken(null),
      });
      return true;
    };
    if (!tryRender()) {
      const t = setInterval(() => { if (tryRender()) clearInterval(t); }, 200);
      return () => clearInterval(t);
    }
    return () => {
      if (widgetIdRef.current !== undefined) w.turnstile?.remove(widgetIdRef.current);
    };
  }, [turnstileSiteKey]);

  const handleSubmit = useCallback(async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (status === "submitting") return;

    const form = e.currentTarget;
    const data = new FormData(form);
    const utm = readUtm();

    setStatus("submitting");

    try {
      const res = await fetch("/api/quotes", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          guestName: data.get("guestName") as string,
          guestPhone: data.get("guestPhone") as string,
          guestEmail: data.get("guestEmail") as string,
          guestCity: data.get("guestCity") as string,
          guestAddress: data.get("guestAddress") as string,
          requirements: data.get("requirements") as string,
          productId,
          productName,
          whereDidYouFindUs: data.get("whereDidYouFindUs") as string,
          utmSource: utm.utm_source,
          utmMedium: utm.utm_medium,
          utmCampaign: utm.utm_campaign,
          referrer: utm.referrer,
          turnstileToken: token ?? "",
        }),
      });
      if (res.ok) {
        setStatus("success");
      } else {
        setStatus("error");
      }
    } catch {
      setStatus("error");
    }
  }, [status, token, productId, productName]);

  if (status === "success") {
    return (
      <div className="rounded-md border border-border bg-muted/50 p-6 text-center">
        <p className="text-lg font-semibold">Inquiry received — we will get back to you shortly.</p>
      </div>
    );
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <h2 className="text-xl font-bold">Made-to-measure inquiry</h2>
      <p className="text-sm text-muted-foreground">
        Describe your requirements and we will prepare a custom quote.
      </p>
      <div className="flex flex-col gap-1">
        <label htmlFor="qf-req" className="text-sm font-medium">Requirements</label>
        <textarea id="qf-req" name="requirements" rows={4} className="rounded-md border border-border bg-background px-3 py-2 text-sm" placeholder="Dimensions, materials, finish…" />
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        <div className="flex flex-col gap-1">
          <label htmlFor="qf-name" className="text-sm font-medium">Name *</label>
          <input id="qf-name" name="guestName" required className="rounded-md border border-border bg-background px-3 py-2 text-sm" />
        </div>
        <div className="flex flex-col gap-1">
          <label htmlFor="qf-phone" className="text-sm font-medium">Phone *</label>
          <input id="qf-phone" name="guestPhone" type="tel" required className="rounded-md border border-border bg-background px-3 py-2 text-sm" />
        </div>
        <div className="flex flex-col gap-1">
          <label htmlFor="qf-email" className="text-sm font-medium">Email</label>
          <input id="qf-email" name="guestEmail" type="email" className="rounded-md border border-border bg-background px-3 py-2 text-sm" />
        </div>
        <div className="flex flex-col gap-1">
          <label htmlFor="qf-city" className="text-sm font-medium">City</label>
          <input id="qf-city" name="guestCity" className="rounded-md border border-border bg-background px-3 py-2 text-sm" />
        </div>
      </div>
      <div className="flex flex-col gap-1">
        <label htmlFor="qf-lead" className="text-sm font-medium">Where did you find us?</label>
        <select id="qf-lead" name="whereDidYouFindUs" className="rounded-md border border-border bg-background px-3 py-2 text-sm">
          <option value="">— select —</option>
          {leadSources.map((s) => (
            <option key={s} value={s}>{s}</option>
          ))}
        </select>
      </div>
      {turnstileSiteKey && <div ref={widgetRef} />}
      {status === "error" && (
        <p className="text-sm text-destructive">Something went wrong — please try again.</p>
      )}
      <button
        type="submit"
        disabled={status === "submitting" || (Boolean(turnstileSiteKey) && !token)}
        className="rounded-md bg-primary px-6 py-3 text-sm font-semibold text-primary-foreground disabled:opacity-50"
      >
        {status === "submitting" ? "Sending…" : "Request a quote"}
      </button>
    </form>
  );
}
