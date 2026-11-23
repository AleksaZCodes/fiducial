import type { Field } from "payload";
import { COMMERCE_CURRENCY } from "../../generated/commerce";

export function priceField(name: string): Field {
  return {
    name,
    type: "group",
    fields: [
      {
        name: "amountMinor",
        type: "number",
        required: true,
        defaultValue: 0,
        admin: {
          description: `Integer minor units. ${COMMERCE_CURRENCY} 1500 = 150000 (exponent 2).`,
        },
      },
      {
        name: "currency",
        type: "text",
        required: true,
        defaultValue: COMMERCE_CURRENCY,
        admin: { readOnly: true },
      },
    ],
  };
}
