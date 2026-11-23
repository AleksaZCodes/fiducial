import type { Page } from "@/payload/payload-types";

type Props = Extract<
  NonNullable<Page["blocks"]>[number],
  { blockType: "product-grid" }
>;

export default function ProductGridBlock({ heading, productSlugs }: Props) {
  return (
    <section className="mx-auto max-w-5xl px-4 py-16">
      {heading && (
        <h2 className="mb-8 text-2xl font-bold tracking-tight">{heading}</h2>
      )}
      {productSlugs && productSlugs.length > 0 && (
        <ul className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4">
          {productSlugs.map((item) => (
            <li key={item.id ?? item.slug}>
              <a
                href={`/products/${item.slug}`}
                className="block rounded-md border p-4 hover:bg-accent"
              >
                {item.slug}
              </a>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
