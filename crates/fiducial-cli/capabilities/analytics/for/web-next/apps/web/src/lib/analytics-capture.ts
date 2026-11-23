"use client";

const UTM_KEY = "fid-utm";

export interface UtmCapture {
  path: string;
  utm_source?: string;
  utm_medium?: string;
  utm_campaign?: string;
  referrer?: string;
}

function capture(): UtmCapture | null {
  if (typeof window === "undefined") return null;
  try {
    const url = new URL(window.location.href);
    const utm: UtmCapture = { path: url.pathname };
    const src = url.searchParams.get("utm_source");
    const med = url.searchParams.get("utm_medium");
    const cam = url.searchParams.get("utm_campaign");
    if (src) utm.utm_source = src;
    if (med) utm.utm_medium = med;
    if (cam) utm.utm_campaign = cam;
    const ref = document.referrer;
    if (ref) {
      try {
        utm.referrer = new URL(ref).hostname;
      } catch {
        utm.referrer = ref.slice(0, 200);
      }
    }
    return utm;
  } catch {
    return null;
  }
}

export function getUtmCapture(): UtmCapture | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.sessionStorage.getItem(UTM_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as UtmCapture;
  } catch {
    return null;
  }
}

export function initAnalyticsCapture(): void {
  if (typeof window === "undefined") return;
  if (window.sessionStorage.getItem(UTM_KEY)) return;
  const utm = capture();
  if (!utm) return;
  try {
    window.sessionStorage.setItem(UTM_KEY, JSON.stringify(utm));
  } catch {
    // sessionStorage unavailable — capture is best-effort.
  }
}
