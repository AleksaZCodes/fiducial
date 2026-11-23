import type { GlobalConfig } from "payload";
import { isOwnerOrStaff, isAuthenticated } from "../access";

export const SiteSettings: GlobalConfig = {
  slug: "site-settings",
  admin: {
    group: "Settings",
  },
  access: {
    read: isAuthenticated,
    update: isOwnerOrStaff,
  },
  fields: [
    {
      name: "siteName",
      type: "text",
      localized: true,
    },
    {
      name: "tagline",
      type: "text",
      localized: true,
    },
    {
      name: "defaultOgImage",
      type: "upload",
      relationTo: "media",
    },
  ],
};
