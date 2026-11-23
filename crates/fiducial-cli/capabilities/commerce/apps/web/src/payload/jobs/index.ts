import type { TaskConfig } from "payload";
import { NoneEmail } from "@fiducial/adapters/email";
import type { Email } from "@fiducial/adapters/email";

async function getEmail(): Promise<Email> {
  try {
    const m = await import("../../../generated/adapters.js");
    return (m.createAdapters as unknown as (e: unknown) => { email: Email })(process.env).email;
  } catch {
    return new NoneEmail();
  }
}

function env(): Record<string, string | undefined> {
  return process.env as unknown as Record<string, string | undefined>;
}

export const notifyNewOrderTask: TaskConfig = {
  slug: "notify-new-order",
  inputSchema: [
    { name: "orderId", type: "text", required: true },
    { name: "ownerEmail", type: "text", required: true },
  ],
  handler: async ({ input, payload }) => {
    const e = env();
    const ownerEmail = (input as Record<string, string>).ownerEmail || e.OWNER_EMAIL;
    if (!ownerEmail) return { output: {} };

    const order = await payload.findByID({
      collection: "orders",
      id: String((input as Record<string, string>).orderId),
    });

    const email = await getEmail();
    const from = `noreply@${e.DOMAIN ?? "example.com"}`;
    const orderId = String(order.id);

    await email
      .send({
        from,
        to: [ownerEmail],
        subject: `New order #${orderId}`,
        html:
          `<p>New order from <strong>${String(order.guestName)}</strong>` +
          (order.guestPhone ? ` (${String(order.guestPhone)})` : "") +
          (order.guestEmail ? `, <a href="mailto:${String(order.guestEmail)}">${String(order.guestEmail)}</a>` : "") +
          `.</p>`,
      })
      .catch(() => {});

    if (order.guestEmail) {
      await email
        .send({
          from,
          to: [String(order.guestEmail)],
          subject: `Order confirmation #${orderId}`,
          html: `<p>Thank you, <strong>${String(order.guestName)}</strong>! We received your order and will be in touch shortly.</p>`,
        })
        .catch(() => {});
    }

    return { output: {} };
  },
};

export const notifyNewQuoteTask: TaskConfig = {
  slug: "notify-new-quote",
  inputSchema: [
    { name: "quoteId", type: "text", required: true },
    { name: "ownerEmail", type: "text", required: true },
  ],
  handler: async ({ input, payload }) => {
    const e = env();
    const ownerEmail = (input as Record<string, string>).ownerEmail || e.OWNER_EMAIL;
    if (!ownerEmail) return { output: {} };

    const quote = await payload.findByID({
      collection: "quotes",
      id: String((input as Record<string, string>).quoteId),
    });

    const email = await getEmail();
    const from = `noreply@${e.DOMAIN ?? "example.com"}`;
    const quoteId = String(quote.id);

    await email
      .send({
        from,
        to: [ownerEmail],
        subject: `New quote request #${quoteId}`,
        html:
          `<p>New quote request from <strong>${String(quote.guestName)}</strong>` +
          (quote.guestPhone ? ` (${String(quote.guestPhone)})` : "") +
          (quote.guestEmail ? `, <a href="mailto:${String(quote.guestEmail)}">${String(quote.guestEmail)}</a>` : "") +
          `.</p>`,
      })
      .catch(() => {});

    if (quote.guestEmail) {
      await email
        .send({
          from,
          to: [String(quote.guestEmail)],
          subject: `Quote request confirmation #${quoteId}`,
          html: `<p>Thank you, <strong>${String(quote.guestName)}</strong>! We received your inquiry and will respond shortly.</p>`,
        })
        .catch(() => {});
    }

    return { output: {} };
  },
};
