import type { Page } from "@/payload/payload-types";
import HeroBlock from "./HeroBlock";
import ProductGridBlock from "./ProductGridBlock";
import FaqBlock from "./FaqBlock";
import CustomOrderCtaBlock from "./CustomOrderCtaBlock";

type Block = NonNullable<Page["blocks"]>[number];

export default function BlockRenderer({ blocks }: { blocks?: Block[] | null }) {
  if (!blocks?.length) return null;
  return (
    <>
      {blocks.map((block, i) => {
        switch (block.blockType) {
          case "hero":
            return <HeroBlock key={i} {...block} />;
          case "product-grid":
            return <ProductGridBlock key={i} {...block} />;
          case "faq":
            return <FaqBlock key={i} {...block} />;
          case "custom-order-cta":
            return <CustomOrderCtaBlock key={i} {...block} />;
          default:
            return null;
        }
      })}
    </>
  );
}
