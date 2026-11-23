import Image from "next/image";
import type { Page } from "@/payload/payload-types";

type Props = Extract<
  NonNullable<Page["blocks"]>[number],
  { blockType: "hero" }
>;

export default function HeroBlock({ heading, subheading, image, ctaLabel, ctaHref }: Props) {
  return (
    <section className="relative overflow-hidden">
      {image && typeof image !== "string" && image.url && (
        <div className="absolute inset-0 -z-10">
          <Image
            src={image.url}
            alt={image.alt ?? ""}
            fill
            sizes="100vw"
            className="object-cover"
            priority
          />
        </div>
      )}
      <div className="mx-auto max-w-5xl px-4 py-24 text-center">
        <h1 className="text-4xl font-bold tracking-tight">{heading}</h1>
        {subheading && <p className="mt-4 text-lg">{subheading}</p>}
        {ctaLabel && ctaHref && (
          <a
            href={ctaHref}
            className="mt-8 inline-block rounded-md bg-primary px-6 py-3 text-sm font-semibold text-primary-foreground"
          >
            {ctaLabel}
          </a>
        )}
      </div>
    </section>
  );
}
