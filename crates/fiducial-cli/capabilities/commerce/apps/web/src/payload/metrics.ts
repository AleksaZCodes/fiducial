import { getPayload } from "payload";
import config from "@payload-config";
import type { Payload } from "payload";

export interface WeeklyOrderRow {
  week: string;
  count: number;
  revenue_minor: number;
}

export interface LeadRow {
  source: string;
  count: number;
}

export interface FunnelRow {
  week: string;
  path: string;
  utm_source: string | null;
  visits: number;
  add_to_cart: number;
  checkout_start: number;
  quote_request: number;
  order_submitted: number;
}

function isoWeek(isoDate: string): string {
  const d = new Date(isoDate);
  const thu = new Date(d);
  thu.setUTCDate(d.getUTCDate() + 4 - (d.getUTCDay() || 7));
  const yearStart = new Date(Date.UTC(thu.getUTCFullYear(), 0, 1));
  const n = Math.ceil(((thu.getTime() - yearStart.getTime()) / 86400000 + 1) / 7);
  return `${thu.getUTCFullYear()}-W${String(n).padStart(2, "0")}`;
}

export async function getWeeklyOrders(payload: Payload): Promise<WeeklyOrderRow[]> {
  const since = new Date(Date.now() - 56 * 24 * 60 * 60 * 1000).toISOString();
  const result = await payload.find({
    collection: "orders",
    limit: 2000,
    where: { createdAt: { greater_than: since } },
    depth: 0,
  });

  const byWeek = new Map<string, WeeklyOrderRow>();
  for (const order of result.docs) {
    const week = isoWeek((order as unknown as { createdAt: string }).createdAt);
    const total = (order as unknown as { total?: { amountMinor?: number } }).total;
    const amount = total?.amountMinor ?? 0;
    const existing = byWeek.get(week) ?? { week, count: 0, revenue_minor: 0 };
    byWeek.set(week, {
      week,
      count: existing.count + 1,
      revenue_minor: existing.revenue_minor + amount,
    });
  }
  return [...byWeek.values()].sort((a, b) => b.week.localeCompare(a.week));
}

export async function getOpenQuotesCount(payload: Payload): Promise<number> {
  const result = await payload.find({
    collection: "quotes",
    limit: 0,
    where: {
      and: [
        { status: { not_equals: "confirmed" } },
        { status: { not_equals: "cancelled" } },
      ],
    },
    depth: 0,
  });
  return result.totalDocs;
}

export async function getLeadsBySource(payload: Payload): Promise<LeadRow[]> {
  const result = await payload.find({
    collection: "orders",
    limit: 2000,
    depth: 0,
  });

  const bySrc = new Map<string, number>();
  for (const order of result.docs) {
    const src = (order as unknown as { leadSource?: string }).leadSource ?? "unknown";
    bySrc.set(src, (bySrc.get(src) ?? 0) + 1);
  }
  return [...bySrc.entries()]
    .map(([source, count]) => ({ source, count }))
    .sort((a, b) => b.count - a.count);
}

export async function getWeeklyFunnel(): Promise<FunnelRow[]> {
  const db = (process.env as unknown as { DB?: D1Database }).DB;
  if (!db) return [];
  const since = Date.now() - 56 * 24 * 60 * 60 * 1000;
  try {
    const result = await db
      .prepare(
        `SELECT
          date(datetime(ts/1000,'unixepoch'),'weekday 1','-6 days') as week,
          path,
          utm_source,
          SUM(CASE WHEN event='page_view'       THEN 1 ELSE 0 END) as visits,
          SUM(CASE WHEN event='add_to_cart'      THEN 1 ELSE 0 END) as add_to_cart,
          SUM(CASE WHEN event='checkout_start'   THEN 1 ELSE 0 END) as checkout_start,
          SUM(CASE WHEN event='quote_request'    THEN 1 ELSE 0 END) as quote_request,
          SUM(CASE WHEN event='order_submitted'  THEN 1 ELSE 0 END) as order_submitted
         FROM analytics_events
         WHERE ts > ?
         GROUP BY week, path, utm_source
         ORDER BY week DESC, visits DESC
         LIMIT 500`,
      )
      .bind(since)
      .all<FunnelRow>();
    return result.results;
  } catch {
    return [];
  }
}

export async function getAllMetrics() {
  const payload = await getPayload({ config });
  const [orders, openQuotes, leads, funnel] = await Promise.all([
    getWeeklyOrders(payload),
    getOpenQuotesCount(payload),
    getLeadsBySource(payload),
    getWeeklyFunnel(),
  ]);
  return { orders, openQuotes, leads, funnel };
}
