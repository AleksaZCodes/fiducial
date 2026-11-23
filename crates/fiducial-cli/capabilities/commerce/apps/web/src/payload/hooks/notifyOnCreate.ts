import type { CollectionAfterChangeHook } from "payload";

function ownerEmail(): string | undefined {
  return (process.env as unknown as Record<string, string | undefined>).OWNER_EMAIL;
}

export const notifyOnCreateOrder: CollectionAfterChangeHook = async ({ doc, operation, req }) => {
  if (operation !== "create") return doc;
  const owner = ownerEmail();
  if (!owner) return doc;
  await req.payload.jobs
    .queue({ task: "notify-new-order" as never, input: { orderId: String(doc.id), ownerEmail: owner } })
    .catch(() => {});
  return doc;
};

export const notifyOnCreateQuote: CollectionAfterChangeHook = async ({ doc, operation, req }) => {
  if (operation !== "create") return doc;
  const owner = ownerEmail();
  if (!owner) return doc;
  await req.payload.jobs
    .queue({ task: "notify-new-quote" as never, input: { quoteId: String(doc.id), ownerEmail: owner } })
    .catch(() => {});
  return doc;
};
