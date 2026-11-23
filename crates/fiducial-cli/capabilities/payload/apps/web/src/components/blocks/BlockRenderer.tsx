import type { Page } from "@/payload/payload-types";
import CustomOrderCtaBlock from "./CustomOrderCtaBlock";
import FaqBlock from "./FaqBlock";
import HeroBlock from "./HeroBlock";
import ProductGridBlock from "./ProductGridBlock";

type Block = NonNullable<Page["blocks"]>[number];

export default function BlockRenderer({ blocks }: { blocks?: Block[] | null }) {
  if (!blocks?.length) return null;
  return (
    <>
      {blocks.map((block) => {
        switch (block.blockType) {
          case "hero":
            return <HeroBlock key={block.id as string} {...block} />;
          case "product-grid":
            return <ProductGridBlock key={block.id as string} {...block} />;
          case "faq":
            return <FaqBlock key={block.id as string} {...block} />;
          case "custom-order-cta":
            return <CustomOrderCtaBlock key={block.id as string} {...block} />;
          default:
            return null;
        }
      })}
    </>
  );
}
