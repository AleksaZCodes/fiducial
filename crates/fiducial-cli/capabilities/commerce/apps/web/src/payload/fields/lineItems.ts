import type { Field } from "payload";
import { priceField } from "./price";

export const lineItemsField: Field = {
  name: "lineItems",
  type: "array",
  fields: [
    {
      name: "productId",
      type: "text",
      admin: { readOnly: true },
    },
    {
      name: "variantId",
      type: "text",
      admin: { readOnly: true },
    },
    {
      name: "productName",
      type: "text",
      required: true,
      localized: true,
    },
    {
      name: "variantName",
      type: "text",
      localized: true,
    },
    {
      name: "quantity",
      type: "number",
      required: true,
      min: 1,
      defaultValue: 1,
    },
    priceField("unitPrice"),
    {
      name: "totalMinor",
      type: "number",
      admin: { readOnly: true },
    },
  ],
};

export const lineItemsJsonField: Field = {
  name: "lineItemsJson",
  type: "textarea",
  admin: {
    description:
      "JSON array: [{productId, variantId, productName, variantName, quantity, unitPriceMinor, currency, totalMinor}]. Used in Orders and Quotes to avoid D1 array-update limitations (Payload issue #15219).",
  },
};
