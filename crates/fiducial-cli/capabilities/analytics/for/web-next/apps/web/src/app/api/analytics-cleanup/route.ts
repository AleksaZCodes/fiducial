import { NextRequest, NextResponse } from "next/server";
import { cleanupAnalyticsEvents } from "../../../../generated/analytics-retention";

export async function POST(req: NextRequest): Promise<NextResponse> {
  const secret = req.headers.get("x-cron-secret");
  const expected = (process.env as unknown as { ANALYTICS_CRON_SECRET?: string })
    .ANALYTICS_CRON_SECRET;
  if (!expected || secret !== expected) {
    return NextResponse.json({ error: "forbidden" }, { status: 403 });
  }

  const db = (req as unknown as { env?: { DB?: D1Database } }).env?.DB;
  if (!db) {
    return NextResponse.json({ deleted: 0, note: "no D1 binding" });
  }

  const deleted = await cleanupAnalyticsEvents(db);
  return NextResponse.json({ deleted });
}
