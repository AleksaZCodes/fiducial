import type { Block } from "payload";

export const Hero: Block = {
  slug: "hero",
  labels: { singular: "Hero", plural: "Heroes" },
  fields: [
    {
      name: "heading",
      type: "text",
      required: true,
      localized: true,
    },
    {
      name: "subheading",
      type: "text",
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
      name: "ctaHref",
      type: "text",
    },
  ],
};
