import { NextRequest, NextResponse } from "next/server";

const MAX_BODY = 4096;

interface EventPayload {
  event: string;
  path: string;
  utm_source?: string;
  utm_medium?: string;
  utm_campaign?: string;
  referrer?: string;
}

function isValidPayload(v: unknown): v is EventPayload {
  if (!v || typeof v !== "object") return false;
  const p = v as Record<string, unknown>;
  return typeof p.event === "string" && typeof p.path === "string";
}

async function visitorHash(
  env: { ANALYTICS_HMAC_SECRET?: string },
  ip: string,
  ua: string,
): Promise<string> {
  const secret = env.ANALYTICS_HMAC_SECRET ?? "";
  const date = new Date().toISOString().slice(0, 10);
  const enc = new TextEncoder();
  const key = await crypto.subtle.importKey(
    "raw",
    enc.encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const sig = await crypto.subtle.sign("HMAC", key, enc.encode(date + ip + ua));
  const bytes = new Uint8Array(sig).slice(0, 16);
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

export async function POST(req: NextRequest): Promise<NextResponse> {
  if (req.headers.get("origin") !== req.nextUrl.origin) {
    return NextResponse.json({ error: "forbidden" }, { status: 403 });
  }

  const contentLength = Number(req.headers.get("content-length") ?? 0);
  if (contentLength > MAX_BODY) {
    return NextResponse.json({ error: "too large" }, { status: 413 });
  }

  let payload: unknown;
  try {
    payload = await req.json();
  } catch {
    return NextResponse.json({ error: "bad json" }, { status: 400 });
  }

  if (!isValidPayload(payload)) {
    return NextResponse.json({ error: "invalid" }, { status: 400 });
  }

  const env = (process as { env: Record<string, string | undefined> }).env;
  const ip = req.headers.get("cf-connecting-ip") ?? req.headers.get("x-forwarded-for") ?? "";
  const ua = req.headers.get("user-agent") ?? "";
  const hash = await visitorHash(env as { ANALYTICS_HMAC_SECRET?: string }, ip, ua);

  try {
    const db = (req as unknown as { env?: { DB?: D1Database } }).env?.DB;
    if (db) {
      await db
        .prepare(
          `INSERT INTO analytics_events
           (visitor_hash, event, path, utm_source, utm_medium, utm_campaign, referrer, ts)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
        )
        .bind(
          hash,
          payload.event,
          payload.path,
          payload.utm_source ?? null,
          payload.utm_medium ?? null,
          payload.utm_campaign ?? null,
          payload.referrer ?? null,
          Date.now(),
        )
        .run();
    }
  } catch {
    // A write failure is not surfaced to the caller — the event is best-effort.
  }

  return NextResponse.json({ ok: true });
}
