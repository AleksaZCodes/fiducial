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
      name: "products",
      type: "relationship",
      relationTo: "products",
      hasMany: true,
    },
  ],
};
