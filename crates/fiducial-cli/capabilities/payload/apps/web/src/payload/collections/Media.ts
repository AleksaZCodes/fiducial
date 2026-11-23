import type { CollectionConfig } from "payload";
import { isOwnerOrStaff, isOwner, isAuthenticated } from "../access";

export const Media: CollectionConfig = {
  slug: "media",
  upload: true,
  admin: {
    useAsTitle: "filename",
  },
  access: {
    create: isOwnerOrStaff,
    read: isAuthenticated,
    update: isOwnerOrStaff,
    delete: isOwner,
  },
  fields: [
    {
      name: "alt",
      type: "text",
      localized: true,
    },
    {
      name: "caption",
      type: "text",
      localized: true,
    },
  ],
};
