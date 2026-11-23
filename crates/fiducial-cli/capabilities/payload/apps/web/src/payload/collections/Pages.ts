import type { CollectionConfig } from "payload";
import { isOwnerOrStaff, isAuthenticated } from "../access";

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
      blocks: [],
    },
  ],
  hooks: {
    afterChange: [
      async ({ doc }) => {
        const { revalidatePath } = await import("next/cache");
        revalidatePath(`/${doc.slug}`);
        revalidatePath("/");
      },
    ],
  },
};
