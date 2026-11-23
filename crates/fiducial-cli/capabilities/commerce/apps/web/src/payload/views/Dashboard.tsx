import React from "react";
import {
  getWeeklyOrders,
  getOpenQuotesCount,
  getLeadsBySource,
  getWeeklyFunnel,
} from "../metrics";
import { getPayload } from "payload";
import config from "@payload-config";

export const Dashboard = async () => {
  const payload = await getPayload({ config });

  const [orders, openQuotes, leads, funnel] = await Promise.all([
    getWeeklyOrders(payload),
    getOpenQuotesCount(payload),
    getLeadsBySource(payload),
    getWeeklyFunnel(),
  ]);

  const tableStyle: React.CSSProperties = {
    borderCollapse: "collapse" as const,
    width: "100%",
    marginBottom: "2rem",
    fontSize: "0.875rem",
  };
  const thStyle: React.CSSProperties = {
    textAlign: "left",
    borderBottom: "2px solid #e5e7eb",
    padding: "0.5rem 0.75rem",
    fontWeight: 600,
  };
  const tdStyle: React.CSSProperties = {
    padding: "0.5rem 0.75rem",
    borderBottom: "1px solid #f3f4f6",
  };
  const sectionStyle: React.CSSProperties = {
    marginBottom: "2.5rem",
  };
  const h2Style: React.CSSProperties = {
    fontSize: "1rem",
    fontWeight: 700,
    marginBottom: "0.75rem",
  };

  return (
    <div style={{ padding: "1.5rem 2rem", maxWidth: "900px" }}>
      <h1 style={{ fontSize: "1.25rem", fontWeight: 700, marginBottom: "2rem" }}>
        Dashboard
      </h1>

      <div style={sectionStyle}>
        <h2 style={h2Style}>Weekly orders</h2>
        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={thStyle}>Week</th>
              <th style={{ ...thStyle, textAlign: "right" }}>Orders</th>
              <th style={{ ...thStyle, textAlign: "right" }}>Revenue (minor units)</th>
            </tr>
          </thead>
          <tbody>
            {orders.length === 0 ? (
              <tr>
                <td colSpan={3} style={tdStyle}>
                  No orders in the last 8 weeks.
                </td>
              </tr>
            ) : (
              orders.map((row) => (
                <tr key={row.week}>
                  <td style={tdStyle}>{row.week}</td>
                  <td style={{ ...tdStyle, textAlign: "right" }}>{row.count}</td>
                  <td style={{ ...tdStyle, textAlign: "right" }}>{row.revenue_minor}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      <div style={sectionStyle}>
        <h2 style={h2Style}>Open quotes</h2>
        <p style={{ fontSize: "2rem", fontWeight: 700 }}>{openQuotes}</p>
      </div>

      <div style={sectionStyle}>
        <h2 style={h2Style}>Leads by source</h2>
        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={thStyle}>Source</th>
              <th style={{ ...thStyle, textAlign: "right" }}>Orders</th>
            </tr>
          </thead>
          <tbody>
            {leads.length === 0 ? (
              <tr>
                <td colSpan={2} style={tdStyle}>
                  No orders yet.
                </td>
              </tr>
            ) : (
              leads.map((row) => (
                <tr key={row.source}>
                  <td style={tdStyle}>{row.source}</td>
                  <td style={{ ...tdStyle, textAlign: "right" }}>{row.count}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      <div style={sectionStyle}>
        <h2 style={h2Style}>Weekly funnel (by landing page)</h2>
        {funnel.length === 0 ? (
          <p style={{ color: "#6b7280", fontSize: "0.875rem" }}>
            No first-party analytics data yet. Add the analytics capability to see the funnel.
          </p>
        ) : (
          <table style={tableStyle}>
            <thead>
              <tr>
                <th style={thStyle}>Week</th>
                <th style={thStyle}>Landing page</th>
                <th style={thStyle}>UTM source</th>
                <th style={{ ...thStyle, textAlign: "right" }}>Visits</th>
                <th style={{ ...thStyle, textAlign: "right" }}>Add to cart</th>
                <th style={{ ...thStyle, textAlign: "right" }}>Checkout start</th>
                <th style={{ ...thStyle, textAlign: "right" }}>Quote request</th>
                <th style={{ ...thStyle, textAlign: "right" }}>Order submitted</th>
              </tr>
            </thead>
            <tbody>
              {funnel.slice(0, 50).map((row, i) => (
                <tr key={i}>
                  <td style={tdStyle}>{row.week}</td>
                  <td style={tdStyle}>{row.path}</td>
                  <td style={tdStyle}>{row.utm_source ?? "—"}</td>
                  <td style={{ ...tdStyle, textAlign: "right" }}>{row.visits}</td>
                  <td style={{ ...tdStyle, textAlign: "right" }}>{row.add_to_cart}</td>
                  <td style={{ ...tdStyle, textAlign: "right" }}>{row.checkout_start}</td>
                  <td style={{ ...tdStyle, textAlign: "right" }}>{row.quote_request}</td>
                  <td style={{ ...tdStyle, textAlign: "right" }}>{row.order_submitted}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
};

export default Dashboard;
