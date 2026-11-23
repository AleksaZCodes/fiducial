import type { CollectionConfig } from "payload";
import { isAuthenticated, isOwnerOrStaff } from "../access";
import { CustomOrderCta } from "../blocks/CustomOrderCta";
import { Faq } from "../blocks/Faq";
import { Hero } from "../blocks/Hero";
import { ProductGrid } from "../blocks/ProductGrid";
import { revalidatePage } from "../revalidate";

export const Pages: CollectionConfig = {
  slug: "pages",
  admin: {
    useAsTitle: "title",
    defaultColumns: ["title", "slug", "updatedAt"],
  },
  access: {
    create: isOwnerOrStaff,
    read: isAuthenticated,
    update: isOwnerOrStaff,
    delete: isOwnerOrStaff,
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
      admin: {
        position: "sidebar",
      },
    },
    {
      name: "blocks",
      type: "blocks",
      localized: true,
      blocks: [Hero, Faq, ProductGrid, CustomOrderCta],
    },
  ],
  hooks: {
    afterChange: [revalidatePage],
  },
};
