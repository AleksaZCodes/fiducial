import { NextRequest, NextResponse } from "next/server";
import { getPayload } from "payload";
import config from "@payload-config";

export const dynamic = "force-dynamic";

export async function POST(req: NextRequest): Promise<NextResponse> {
  const env = process.env as unknown as Record<string, string | undefined>;
  const secret = env.CF_CRON_SECRET;
  if (secret) {
    const auth = req.headers.get("authorization") ?? "";
    if (auth !== `Bearer ${secret}`) {
      return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
    }
  }
  const payload = await getPayload({ config });
  const result = await payload.jobs.run({ limit: 25 });
  return NextResponse.json({ ok: true, ran: result });
}
