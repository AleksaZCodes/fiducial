import type { CollectionConfig } from "payload";
import { isOwner, isOwnerOrStaff, isAuthenticated } from "../access";
import { priceField } from "../fields/price";

export const Products: CollectionConfig = {
  slug: "products",
  admin: {
    useAsTitle: "title",
    defaultColumns: ["title", "slug", "active", "updatedAt"],
    group: "Commerce",
  },
  access: {
    create: isOwnerOrStaff,
    read: isAuthenticated,
    update: isOwnerOrStaff,
    delete: isOwner,
  },
  versions: {
    drafts: true,
  },
  fields: [
    {
      name: "title",
      type: "text",
      required: true,
      localized: true,
    },
    {
      name: "slug",
      type: "text",
      required: true,
      unique: true,
      admin: { position: "sidebar" },
    },
    {
      name: "description",
      type: "richText",
      localized: true,
    },
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
    priceField("price"),
    {
      name: "active",
      type: "checkbox",
      defaultValue: true,
      admin: { position: "sidebar" },
    },
    {
      name: "featured",
      type: "checkbox",
      defaultValue: false,
      admin: { position: "sidebar" },
    },
  ],
};
