import type { CollectionConfig } from "payload";
import { isOwner, isOwnerOrStaff, isAuthenticated } from "../access";
import { priceField } from "../fields/price";

export const Variants: CollectionConfig = {
  slug: "variants",
  admin: {
    useAsTitle: "title",
    defaultColumns: ["title", "product", "active", "updatedAt"],
    group: "Commerce",
  },
  access: {
    create: isOwnerOrStaff,
    read: isAuthenticated,
    update: isOwnerOrStaff,
    delete: isOwner,
  },
  fields: [
    {
      name: "product",
      type: "relationship",
      relationTo: "products",
      required: true,
      admin: { position: "sidebar" },
    },
    {
      name: "title",
      type: "text",
      required: true,
      localized: true,
    },
    {
      name: "sku",
      type: "text",
      unique: true,
      admin: { position: "sidebar" },
    },
    {
      type: "collapsible",
      label: "Options",
      fields: [
        {
          name: "dimensions",
          type: "text",
          localized: true,
          admin: { description: "e.g. 120×60×75 cm" },
        },
        {
          name: "material",
          type: "text",
          localized: true,
          admin: { description: "e.g. Solid oak" },
        },
        {
          name: "color",
          type: "text",
          localized: true,
          admin: { description: "e.g. Natural / Walnut stain" },
        },
      ],
    },
    priceField("price"),
    {
      name: "images",
      type: "array",
      fields: [
        {
          name: "image",
          type: "upload",
          relationTo: "media",
          required: true,
        },
        {
          name: "alt",
          type: "text",
          localized: true,
        },
      ],
    },
    {
      name: "active",
      type: "checkbox",
      defaultValue: true,
      admin: { position: "sidebar" },
    },
  ],
};
