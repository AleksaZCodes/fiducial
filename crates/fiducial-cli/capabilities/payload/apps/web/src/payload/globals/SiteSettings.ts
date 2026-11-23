import type { GlobalConfig } from "payload";
import { isOwnerOrStaff, isAuthenticated } from "../access";
import { revalidateSiteSettings } from "../revalidate";

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
    {
      type: "collapsible",
      label: "Company Info",
      fields: [
        {
          name: "companyName",
          type: "text",
        },
        {
          name: "address",
          type: "textarea",
        },
        {
          name: "phone",
          type: "text",
        },
        {
          name: "email",
          type: "email",
        },
        {
          name: "pib",
          type: "text",
          admin: { description: "Tax ID (PIB) for Serbian legal pages" },
        },
      ],
    },
    {
      type: "collapsible",
      label: "Navigation",
      fields: [
        {
          name: "mainNav",
          type: "array",
          fields: [
            {
              name: "label",
              type: "text",
              required: true,
              localized: true,
            },
            {
              name: "href",
              type: "text",
              required: true,
            },
          ],
        },
        {
          name: "footerNav",
          type: "array",
          fields: [
            {
              name: "label",
              type: "text",
              required: true,
              localized: true,
            },
            {
              name: "href",
              type: "text",
              required: true,
            },
          ],
        },
      ],
    },
    {
      type: "collapsible",
      label: "Banners",
      fields: [
        {
          name: "banner",
          type: "group",
          fields: [
            {
              name: "enabled",
              type: "checkbox",
              defaultValue: false,
            },
            {
              name: "text",
              type: "text",
              localized: true,
            },
            {
              name: "href",
              type: "text",
            },
          ],
        },
      ],
    },
    {
      type: "collapsible",
      label: "SEO Defaults",
      fields: [
        {
          name: "seoTitle",
          type: "text",
          localized: true,
        },
        {
          name: "seoDescription",
          type: "textarea",
          localized: true,
        },
        {
          name: "twitterHandle",
          type: "text",
        },
      ],
    },
  ],
  hooks: {
    afterChange: [revalidateSiteSettings],
  },
};
