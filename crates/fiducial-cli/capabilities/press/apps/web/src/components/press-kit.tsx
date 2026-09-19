import type * as React from "react";

import { type PressLocale, press, pressFor } from "@/generated/press";

/**
 * The press room: guidelines, kit, and stories.
 *
 * Everything here is read from `generated/press.ts`, which `fid derive` builds
 * from `press/` and `fiducial.toml`. Nothing on this page is typed twice —
 * least of all the logo, which comes from the two brand primitives the rest of
 * the product already uses.
 *
 * ## Why the copy blocks are `<textarea readOnly>` and not `<p>`
 *
 * A boilerplate paragraph exists to be taken. Rendering it as prose means a
 * journalist selects it with a mouse, drags past the edges, and pastes markup
 * or a truncated sentence. A read-only field selects cleanly on click and
 * copies as plain text, which is the entire job.
 */
export function PressKit({
  locale,
  labels,
  logos,
}: {
  /**
   * Which locale's press room to render.
   *
   * Required, and there is no fallback. A locale missing its boilerplate fails
   * `fid derive`, so by the time this renders every declared locale is
   * complete — which is why this can index without a guard.
   */
  locale: PressLocale;
  /** Localized headings. The capability does not own this product's copy. */
  labels: Record<
    | "title"
    | "boilerplate"
    | "facts"
    | "assets"
    | "stories"
    | "contact"
    | "copy"
    | "short"
    | "medium"
    | "long"
    | "guidelines",
    string
  >;
  /** Downloadable marks. These are the brand primitives, not new files. */
  logos: { label: string; href: string }[];
}) {
  const room = pressFor(locale);

  return (
    <div className="mx-auto max-w-4xl px-5 py-20 sm:px-8">
      <h1 className="type-h1">{labels.title}</h1>

      <section className="mt-14">
        <h2 className="type-h2">{labels.boilerplate}</h2>
        <div className="mt-6 flex flex-col gap-6">
          {(["short", "medium", "long"] as const).map((k) =>
            room.boilerplate[k] ? (
              <div key={k}>
                <h3 className="type-small font-display font-semibold">{labels[k]}</h3>
                {/* `bg-transparent` is load-bearing: a textarea paints its own
                    background, and a chamfered box paints its fill on a
                    pseudo-element behind the content. Leave the default and the
                    control renders as a grey slab sitting on top of the surface.

                    `field-sizing: content` grows the box to its text, with a
                    generous `rows` as the fallback where it is unsupported —
                    boilerplate that is cut off at the bottom is worse than no
                    boilerplate, because it copies silently truncated. */}
                <textarea
                  readOnly
                  aria-label={labels[k]}
                  rows={k === "long" ? 9 : k === "medium" ? 5 : 3}
                  style={{ fieldSizing: "content" } as React.CSSProperties}
                  className="cham type-small mt-2 w-full resize-none bg-transparent p-4 font-body text-foreground"
                  value={room.boilerplate[k]}
                />
              </div>
            ) : null,
          )}
        </div>
      </section>

      <section className="mt-14">
        <h2 className="type-h2">{labels.facts}</h2>
        <dl className="mt-6 grid gap-x-8 gap-y-3 sm:grid-cols-2">
          {room.facts.map((f) => (
            <div key={f.label} className="flex justify-between gap-4 border-b border-border pb-2">
              <dt className="type-small text-muted-foreground">{f.label}</dt>
              <dd className="type-small font-medium">{f.value}</dd>
            </div>
          ))}
        </dl>
      </section>

      <section className="mt-14">
        <h2 className="type-h2">{labels.assets}</h2>
        <p className="type-small mt-3 max-w-[60ch] text-muted-foreground">{labels.guidelines}</p>
        <ul className="mt-6 flex flex-wrap gap-3">
          {logos.map((l) => (
            <li key={l.href}>
              <a
                href={l.href}
                download
                className="cham-sm cham-outline type-small inline-block px-4 py-2 font-medium"
              >
                {l.label}
              </a>
            </li>
          ))}
        </ul>
      </section>

      <section className="mt-14">
        <h2 className="type-h2">{labels.stories}</h2>
        <ul className="mt-6 flex flex-col gap-4">
          {room.stories.map((s) => (
            <li key={s.slug} className="cham p-6">
              <div className="flex items-baseline justify-between gap-4">
                <h3 className="type-h3">{s.title}</h3>
                {/* The angle is the point of the index: it is what a reporter
                    scans for when deciding which story fits their section. */}
                {s.angle ? (
                  <span className="type-mono shrink-0 text-muted-foreground">{s.angle}</span>
                ) : null}
              </div>
              {s.summary ? (
                <p className="type-small mt-2 max-w-[60ch] text-muted-foreground">{s.summary}</p>
              ) : null}
            </li>
          ))}
        </ul>
      </section>

      <section className="mt-14">
        <h2 className="type-h2">{labels.contact}</h2>
        <p className="type-mono mt-3">
          <a href={`mailto:${press.pressEmail}`} className="underline-offset-4 hover:underline">
            {press.pressEmail}
          </a>
        </p>
        {press.spokesperson ? (
          <p className="type-small mt-2 text-muted-foreground">{press.spokesperson}</p>
        ) : null}
      </section>
    </div>
  );
}
