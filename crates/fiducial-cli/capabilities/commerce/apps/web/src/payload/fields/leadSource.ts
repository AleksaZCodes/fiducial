import type { Field } from "payload";
import { COMMERCE_LEAD_SOURCES } from "../../generated/commerce";

export const leadSourceField: Field = {
  name: "leadSource",
  type: "select",
  options: COMMERCE_LEAD_SOURCES.map((s) => ({ label: s, value: s })),
  admin: { position: "sidebar" },
};

export const utmFields: Field[] = [
  {
    name: "utmSource",
    type: "text",
    admin: { readOnly: true, position: "sidebar" },
  },
  {
    name: "utmMedium",
    type: "text",
    admin: { readOnly: true, position: "sidebar" },
  },
  {
    name: "utmCampaign",
    type: "text",
    admin: { readOnly: true, position: "sidebar" },
  },
  {
    name: "utmTerm",
    type: "text",
    admin: { readOnly: true, position: "sidebar" },
  },
  {
    name: "utmContent",
    type: "text",
    admin: { readOnly: true, position: "sidebar" },
  },
];
