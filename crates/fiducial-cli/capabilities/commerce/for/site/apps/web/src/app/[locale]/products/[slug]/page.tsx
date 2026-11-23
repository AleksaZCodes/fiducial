import { notFound } from "next/navigation";
import { getPayload } from "payload";
import config from "@payload-config";
import { isLocale } from "@/lib/routes";
import { COMMERCE_LEAD_SOURCES } from "@/generated/commerce";
import VariantPicker from "@/components/commerce/VariantPicker";
import OrderForm from "@/components/commerce/OrderForm";
import QuoteForm from "@/components/commerce/QuoteForm";

export const dynamic = "force-dynamic";

type Props = { params: Promise<{ locale: string; slug: string }> };

export default async function ProductPage({ params }: Props) {
  const { locale, slug } = await params;
  if (!isLocale(locale)) notFound();

  const payload = await getPayload({ config });

  const products = await payload.find({
    collection: "products",
    where: { slug: { equals: slug }, active: { equals: true } },
    locale: locale as never,
    depth: 2,
    limit: 1,
  });

  if (!products.docs.length) notFound();
  const product = products.docs[0] as unknown as {
    id: string;
    title: string;
    description?: unknown;
    price?: { amountMinor?: number; currency?: string };
  };

  const variantResult = await payload.find({
    collection: "variants",
    where: {
      product: { equals: product.id },
      active: { equals: true },
    },
    locale: locale as never,
    depth: 1,
    limit: 100,
  });

  const variants = (variantResult.docs as unknown[]).map((v) => {
    const vt = v as {
      id: string;
      title: string;
      dimensions?: string;
      material?: string;
      color?: string;
      price?: { amountMinor?: number; currency?: string };
    };
    return {
      id: String(vt.id),
      title: String(vt.title),
      dimensions: vt.dimensions ?? null,
      material: vt.material ?? null,
      color: vt.color ?? null,
      price: vt.price ?? null,
      unitPriceMinor: vt.price?.amountMinor,
      currency: vt.price?.currency,
    };
  });

  const siteKey = (process.env as unknown as { NEXT_PUBLIC_TURNSTILE_SITE_KEY?: string })
    .NEXT_PUBLIC_TURNSTILE_SITE_KEY ?? "";

  return (
    <div className="mx-auto max-w-5xl px-4 py-16">
      <h1 className="text-3xl font-bold">{String(product.title)}</h1>

      {product.price?.amountMinor != null && (
        <p className="mt-2 text-xl font-semibold text-muted-foreground">
          {(product.price.amountMinor / 100).toLocaleString()} {product.price.currency}
        </p>
      )}

      {variants.length > 0 && (
        <VariantPicker variants={variants} />
      )}

      <div className="mt-16 grid gap-16 lg:grid-cols-2">
        <section id="order-form" className="scroll-mt-8">
          <OrderForm
            productId={String(product.id)}
            productName={String(product.title)}
            variants={variants}
            leadSources={COMMERCE_LEAD_SOURCES}
            turnstileSiteKey={siteKey}
          />
        </section>

        <section id="quote-form" className="scroll-mt-8">
          <QuoteForm
            productId={String(product.id)}
            productName={String(product.title)}
            leadSources={COMMERCE_LEAD_SOURCES}
            turnstileSiteKey={siteKey}
          />
        </section>
      </div>
    </div>
  );
}
