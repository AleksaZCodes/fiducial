import type { CollectionConfig } from "payload";
import { isOwner, isOwnerOrStaff } from "../access";

export const Discounts: CollectionConfig = {
  slug: "discounts",
  admin: {
    useAsTitle: "code",
    defaultColumns: ["code", "type", "active"],
    group: "Commerce",
    hidden: true,
  },
  access: {
    create: isOwnerOrStaff,
    read: isOwnerOrStaff,
    update: isOwnerOrStaff,
    delete: isOwner,
  },
  fields: [
    {
      name: "code",
      type: "text",
      required: true,
      unique: true,
    },
    {
      name: "type",
      type: "select",
      required: true,
      options: [
        { label: "Percentage", value: "percentage" },
        { label: "Fixed (minor units)", value: "fixed" },
      ],
    },
    {
      name: "amountMinor",
      type: "number",
      required: true,
      defaultValue: 0,
      admin: {
        description:
          "For percentage: 0–100. For fixed: amount in minor units (e.g. 50000 = 500 RSD).",
      },
    },
    {
      name: "active",
      type: "checkbox",
      defaultValue: true,
      admin: { position: "sidebar" },
    },
    {
      name: "note",
      type: "textarea",
      admin: { description: "Internal note — not shown to customers." },
    },
  ],
};
