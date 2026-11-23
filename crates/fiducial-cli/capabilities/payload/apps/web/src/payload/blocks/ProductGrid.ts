import type { Block } from "payload";

export const ProductGrid: Block = {
  slug: "product-grid",
  labels: { singular: "Product Grid", plural: "Product Grids" },
  fields: [
    {
      name: "heading",
      type: "text",
      localized: true,
    },
    {
      name: "productSlugs",
      type: "array",
      admin: {
        description:
          "Comma-separated product slugs. When the commerce capability is installed these become a relationship field.",
      },
      fields: [
        {
          name: "slug",
          type: "text",
          required: true,
        },
      ],
    },
  ],
};
