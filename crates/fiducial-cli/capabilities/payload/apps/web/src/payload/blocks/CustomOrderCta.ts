import type { Block } from "payload";

export const CustomOrderCta: Block = {
  slug: "custom-order-cta",
  labels: { singular: "Custom Order CTA", plural: "Custom Order CTAs" },
  fields: [
    {
      name: "heading",
      type: "text",
      required: true,
      localized: true,
    },
    {
      name: "body",
      type: "richText",
      localized: true,
    },
    {
      name: "image",
      type: "upload",
      relationTo: "media",
    },
    {
      name: "ctaLabel",
      type: "text",
      localized: true,
    },
    {
      name: "formAnchor",
      type: "text",
      admin: {
        description: "The id of the custom-order form on this page",
      },
    },
  ],
};
