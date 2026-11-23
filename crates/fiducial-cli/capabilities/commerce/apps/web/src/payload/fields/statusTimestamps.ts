import type { Field, CollectionBeforeChangeHook } from "payload";
import { COMMERCE_STATUSES } from "../../generated/commerce";

export const statusTimestampFields: Field[] = COMMERCE_STATUSES.map((status) => ({
  name: `${status}At`,
  type: "date" as const,
  admin: {
    readOnly: true,
    position: "sidebar" as const,
    date: { displayFormat: "d MMM yyyy HH:mm" },
  },
}));

export const setStatusTimestamp: CollectionBeforeChangeHook = async ({ data, originalDoc }) => {
  if (!data.status) return data;
  const prevStatus = originalDoc?.status;
  if (data.status === prevStatus) return data;
  const field = `${data.status}At`;
  if (data[field]) return data;
  return { ...data, [field]: new Date().toISOString() };
};
