import { NextRequest, NextResponse } from "next/server";
import { getPayload } from "payload";
import config from "@payload-config";
import { Turnstile, NoneBotProtection } from "@fiducial/adapters/bot-protection";
import type { Analytics } from "@fiducial/adapters/analytics";
import { NoneAnalytics } from "@fiducial/adapters/analytics";

const MAX_BODY = 8192;

interface OrderRequest {
  guestName: string;
  guestPhone: string;
  guestEmail?: string;
  guestCity?: string;
  guestAddress?: string;
  productId: string;
  variantId?: string;
  productName: string;
  variantName?: string;
  unitPriceMinor?: number;
  currency?: string;
  quantity?: number;
  leadSource?: string;
  whereDidYouFindUs?: string;
  utmSource?: string;
  utmMedium?: string;
  utmCampaign?: string;
  referrer?: string;
  turnstileToken: string;
}

function isValidRequest(v: unknown): v is OrderRequest {
  if (!v || typeof v !== "object") return false;
  const r = v as Record<string, unknown>;
  return typeof r.guestName === "string" && r.guestName.length > 0
    && typeof r.guestPhone === "string" && r.guestPhone.length > 0
    && typeof r.productId === "string"
    && typeof r.productName === "string"
    && typeof r.turnstileToken === "string";
}

async function getAnalytics(): Promise<Analytics> {
  try {
    const m = await import("../../../generated/analytics.js");
    return m.createAnalytics(process.env);
  } catch {
    return new NoneAnalytics();
  }
}

export async function POST(req: NextRequest): Promise<NextResponse> {
  const contentLength = Number(req.headers.get("content-length") ?? 0);
  if (contentLength > MAX_BODY) {
    return NextResponse.json({ error: "too large" }, { status: 413 });
  }

  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "bad json" }, { status: 400 });
  }

  if (!isValidRequest(body)) {
    return NextResponse.json({ error: "invalid" }, { status: 400 });
  }

  const env = process.env as unknown as Record<string, string | undefined>;
  const secretKey = env.TURNSTILE_SECRET_KEY;
  const bp = secretKey ? new Turnstile({ TURNSTILE_SECRET_KEY: secretKey }) : new NoneBotProtection();
  const ip = req.headers.get("cf-connecting-ip") ?? req.headers.get("x-forwarded-for") ?? undefined;

  const outcome = await bp.verify(body.turnstileToken, ip);
  if (!outcome.success) {
    return NextResponse.json({ error: "challenge failed" }, { status: 400 });
  }

  const quantity = body.quantity ?? 1;
  const unitPriceMinor = body.unitPriceMinor ?? 0;
  const currency = body.currency ?? "RSD";
  const lineItem = {
    productId: body.productId,
    variantId: body.variantId ?? null,
    productName: body.productName,
    variantName: body.variantName ?? null,
    quantity,
    unitPriceMinor,
    currency,
    totalMinor: unitPriceMinor * quantity,
  };

  const payload = await getPayload({ config });
  const order = await payload.create({
    collection: "orders",
    data: {
      status: "new",
      guestName: body.guestName,
      guestPhone: body.guestPhone,
      guestEmail: body.guestEmail,
      guestCity: body.guestCity,
      guestAddress: body.guestAddress,
      lineItemsJson: JSON.stringify([lineItem]),
      total: { amountMinor: lineItem.totalMinor, currency },
      leadSource: body.whereDidYouFindUs ?? body.leadSource,
      utmSource: body.utmSource,
      utmMedium: body.utmMedium,
      utmCampaign: body.utmCampaign,
    },
  });

  const path = req.nextUrl.pathname;
  const analytics = await getAnalytics();
  await analytics.track({
    name: "order_submitted",
    path,
    utm_source: body.utmSource,
    utm_medium: body.utmMedium,
    utm_campaign: body.utmCampaign,
    referrer: body.referrer,
  });

  return NextResponse.json({ ok: true, orderId: order.id });
}
