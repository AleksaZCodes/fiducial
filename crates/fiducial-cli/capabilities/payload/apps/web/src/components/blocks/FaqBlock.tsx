import type { Page } from "@/payload/payload-types";

type Props = Extract<
  NonNullable<Page["blocks"]>[number],
  { blockType: "faq" }
>;

export default function FaqBlock({ heading, items }: Props) {
  return (
    <section className="mx-auto max-w-3xl px-4 py-16">
      {heading && (
        <h2 className="mb-8 text-2xl font-bold tracking-tight">{heading}</h2>
      )}
      <dl className="space-y-6">
        {items?.map((item) => (
          <div key={item.id}>
            <dt className="font-semibold">{item.question}</dt>
            {item.answer && (
              <dd className="mt-2 text-muted-foreground">
                {typeof item.answer === "string" ? item.answer : null}
              </dd>
            )}
          </div>
        ))}
      </dl>
    </section>
  );
}
