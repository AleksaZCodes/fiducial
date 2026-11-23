import type { CollectionConfig } from "payload";
import { isOwner, isOwnerOrStaff } from "../access";
import { leadSourceField, utmFields } from "../fields/leadSource";
import { lineItemsField } from "../fields/lineItems";

export const Carts: CollectionConfig = {
  slug: "carts",
  admin: {
    useAsTitle: "sessionId",
    defaultColumns: ["sessionId", "updatedAt"],
    group: "Commerce",
    description: "Guest carts. Enabled in the UI when [commerce] cart = true.",
  },
  access: {
    create: () => true,
    read: isOwnerOrStaff,
    update: () => true,
    delete: isOwner,
  },
  fields: [
    {
      name: "sessionId",
      type: "text",
      required: true,
      unique: true,
      admin: { position: "sidebar" },
    },
    {
      type: "collapsible",
      label: "Guest",
      fields: [
        { name: "guestName", type: "text" },
        { name: "guestPhone", type: "text" },
        { name: "guestEmail", type: "email" },
        { name: "guestCity", type: "text" },
        { name: "guestAddress", type: "textarea" },
      ],
    },
    lineItemsField,
    leadSourceField,
    ...utmFields,
  ],
};
