import Image from "next/image";
import type { Page } from "@/payload/payload-types";

type Props = Extract<
  NonNullable<Page["blocks"]>[number],
  { blockType: "custom-order-cta" }
>;

export default function CustomOrderCtaBlock({
  heading,
  image,
  ctaLabel,
  formAnchor,
}: Props) {
  return (
    <section className="mx-auto max-w-5xl px-4 py-16">
      <div className="flex flex-col gap-8 md:flex-row md:items-center">
        {image && typeof image !== "string" && image.url && (
          <div className="relative h-64 w-full md:h-80 md:w-1/2 shrink-0">
            <Image
              src={image.url}
              alt={image.alt ?? ""}
              fill
              sizes="(max-width: 768px) 100vw, 50vw"
              className="rounded-md object-cover"
            />
          </div>
        )}
        <div>
          <h2 className="text-2xl font-bold tracking-tight">{heading}</h2>
          {ctaLabel && formAnchor && (
            <a
              href={`#${formAnchor}`}
              className="mt-6 inline-block rounded-md bg-primary px-6 py-3 text-sm font-semibold text-primary-foreground"
            >
              {ctaLabel}
            </a>
          )}
        </div>
      </div>
    </section>
  );
}
