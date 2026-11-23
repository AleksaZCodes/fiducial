"use client";

import { type FormEvent, useCallback, useEffect, useRef, useState } from "react";
import { useSearchParams } from "next/navigation";
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

interface VariantInfo {
  id: string;
  title: string;
  unitPriceMinor?: number;
  currency?: string;
}

interface OrderFormProps {
  productId: string;
  productName: string;
  variants: VariantInfo[];
  leadSources: readonly CommerceLeadSource[];
  turnstileSiteKey: string;
}

type Status = "idle" | "submitting" | "success" | "error";

export default function OrderForm({
  productId,
  productName,
  variants,
  leadSources,
  turnstileSiteKey,
}: OrderFormProps) {
  const searchParams = useSearchParams();
  const selectedVariantId = searchParams.get("variant") ?? (variants[0]?.id ?? "");
  const selectedVariant = variants.find((v) => v.id === selectedVariantId) ?? variants[0];

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

    try {
      fetch("/api/e", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ event: "checkout_start", path: window.location.pathname }),
      });
    } catch {}

    setStatus("submitting");

    try {
      const res = await fetch("/api/orders", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          guestName: data.get("guestName") as string,
          guestPhone: data.get("guestPhone") as string,
          guestEmail: data.get("guestEmail") as string,
          guestCity: data.get("guestCity") as string,
          guestAddress: data.get("guestAddress") as string,
          productId,
          productName,
          variantId: selectedVariant?.id,
          variantName: selectedVariant?.title,
          unitPriceMinor: selectedVariant?.unitPriceMinor,
          currency: selectedVariant?.currency ?? "RSD",
          quantity: 1,
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
  }, [status, token, productId, productName, selectedVariant]);

  if (status === "success") {
    return (
      <div className="rounded-md border border-border bg-muted/50 p-6 text-center">
        <p className="text-lg font-semibold">Order received — we will call you to confirm.</p>
      </div>
    );
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <h2 className="text-xl font-bold">Place an order</h2>
      {selectedVariant && (
        <p className="text-sm text-muted-foreground">
          {productName}{selectedVariant.title !== productName ? ` — ${selectedVariant.title}` : ""}
        </p>
      )}
      <div className="grid gap-4 sm:grid-cols-2">
        <div className="flex flex-col gap-1">
          <label htmlFor="of-name" className="text-sm font-medium">Name *</label>
          <input id="of-name" name="guestName" required className="rounded-md border border-border bg-background px-3 py-2 text-sm" />
        </div>
        <div className="flex flex-col gap-1">
          <label htmlFor="of-phone" className="text-sm font-medium">Phone *</label>
          <input id="of-phone" name="guestPhone" type="tel" required className="rounded-md border border-border bg-background px-3 py-2 text-sm" />
        </div>
        <div className="flex flex-col gap-1">
          <label htmlFor="of-email" className="text-sm font-medium">Email</label>
          <input id="of-email" name="guestEmail" type="email" className="rounded-md border border-border bg-background px-3 py-2 text-sm" />
        </div>
        <div className="flex flex-col gap-1">
          <label htmlFor="of-city" className="text-sm font-medium">City</label>
          <input id="of-city" name="guestCity" className="rounded-md border border-border bg-background px-3 py-2 text-sm" />
        </div>
      </div>
      <div className="flex flex-col gap-1">
        <label htmlFor="of-address" className="text-sm font-medium">Address</label>
        <textarea id="of-address" name="guestAddress" rows={2} className="rounded-md border border-border bg-background px-3 py-2 text-sm" />
      </div>
      <div className="flex flex-col gap-1">
        <label htmlFor="of-lead" className="text-sm font-medium">Where did you find us?</label>
        <select id="of-lead" name="whereDidYouFindUs" className="rounded-md border border-border bg-background px-3 py-2 text-sm">
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
        {status === "submitting" ? "Sending…" : "Place order"}
      </button>
    </form>
  );
}
