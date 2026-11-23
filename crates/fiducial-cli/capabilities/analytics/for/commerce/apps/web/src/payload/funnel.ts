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

export async function getWeeklyFunnel(db: D1Database, weeks = 8): Promise<FunnelRow[]> {
  const since = Date.now() - weeks * 7 * 24 * 60 * 60 * 1000;
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
}
