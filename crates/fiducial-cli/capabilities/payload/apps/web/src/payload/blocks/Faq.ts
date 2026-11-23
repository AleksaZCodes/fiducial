import type { Block } from "payload";

export const Faq: Block = {
  slug: "faq",
  labels: { singular: "FAQ", plural: "FAQs" },
  fields: [
    {
      name: "heading",
      type: "text",
      localized: true,
    },
    {
      name: "items",
      type: "array",
      required: true,
      minRows: 1,
      fields: [
        {
          name: "question",
          type: "text",
          required: true,
          localized: true,
        },
        {
          name: "answer",
          type: "richText",
          localized: true,
        },
      ],
    },
  ],
};
