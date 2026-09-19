import { MediaFigure } from "@/components/media-figure";
import { type PressLocale, press, pressFor } from "@/generated/press";
import { mediaUrl } from "@/lib/media";

/**
 * The press room.
 *
 * ## The order is the order a journalist needs things in
 *
 *   1. **Assets** — the logo and the gallery. Usually the reason they came:
 *      they already have a story and need a picture for it.
 *   2. **Fast facts** — the table they check their copy against.
 *   3. **Boilerplate** — the paragraph they paste at the bottom.
 *   4. **Stories** — for the ones who have not got an angle yet.
 *   5. **Contact** — last, because by now they know what to ask.
 *
 * It was ordered the other way round, prose first, which put the one thing
 * most visitors wanted below three screens of text.
 *
 * Every image carries a caption and a download button (see MediaFigure), and
 * a story card is a single link — the whole card, not just its title — because
 * a card that looks clickable and only responds on one line is a card people
 * click on the wrong part of.
 *
 * Plain `<a>` elements and no framework import beyond JSX, so the port to
 * another framework is a markup change.
 */
export function PressKit({
  locale,
  labels,
  logos,
  gallery,
  storyHref,
  angleLabel,
  formatDate,
}: {
  locale: PressLocale;
  labels: Record<
    | "title"
    | "assets"
    | "guidelines"
    | "facts"
    | "boilerplate"
    | "short"
    | "medium"
    | "long"
    | "stories"
    | "contact"
    | "download",
    string
  >;
  /** The brand primitives, previewed and downloadable. Served from the repo, not the bucket. */
  logos: { label: string; href: string }[];
  /** Pictures from the media bucket, with their localized captions. */
  gallery: readonly { key: string; alt: string; caption: string }[];
  storyHref: (slug: string) => string;
  angleLabel: (angle: string) => string;
  formatDate: (iso: string) => string;
}) {
  const room = pressFor(locale);

  return (
    <div className="flex flex-col gap-14">
      <h1 className="type-h1">{labels.title}</h1>

      <section className="flex flex-col gap-6">
        <h2 className="type-h2">{labels.assets}</h2>
        <p className="type-body text-muted-foreground">{labels.guidelines}</p>

        <ul className="grid gap-6 sm:grid-cols-2">
          {logos.map((l) => (
            <li key={l.href} className="flex flex-col gap-3">
              <div className="cham flex aspect-[3/2] items-center justify-center p-8">
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img src={l.href} alt={l.label} className="h-full w-full object-contain" />
              </div>
              <div className="flex items-start justify-between gap-4">
                <span className="type-small text-muted-foreground">{l.label}</span>
                <a
                  href={l.href}
                  download
                  className="cham-sm cham-outline type-small shrink-0 px-3 py-1.5 font-medium"
                >
                  {labels.download}
                </a>
              </div>
            </li>
          ))}
          {gallery.map((g) => (
            <li key={g.key}>
              <MediaFigure
                mediaKey={g.key}
                alt={g.alt}
                caption={g.caption}
                downloadLabel={labels.download}
                fit="contain"
              />
            </li>
          ))}
        </ul>
      </section>

      <section className="flex flex-col gap-6">
        <h2 className="type-h2">{labels.facts}</h2>
        <dl className="cham grid gap-x-8 p-6 sm:grid-cols-2">
          {room.facts.map((f) => (
            <div
              key={f.label}
              className="flex min-w-0 flex-col gap-0.5 border-b border-border py-3"
            >
              <dt className="type-small text-muted-foreground">{f.label}</dt>
              <dd className="type-body break-words">{f.value}</dd>
            </div>
          ))}
        </dl>
      </section>

      <section className="flex flex-col gap-6">
        <h2 className="type-h2">{labels.boilerplate}</h2>
        {(["short", "medium", "long"] as const).map((k) =>
          room.boilerplate[k] ? (
            <div key={k} className="flex flex-col gap-2">
              <h3 className="type-small font-display font-semibold">{labels[k]}</h3>
              {/* Read-only field rather than a paragraph: it selects cleanly on
                  click and copies as plain text, which is the whole job. */}
              <textarea
                readOnly
                aria-label={labels[k]}
                rows={k === "long" ? 9 : k === "medium" ? 5 : 3}
                style={{ fieldSizing: "content" } as React.CSSProperties}
                className="cham type-body w-full resize-none bg-transparent p-4 text-foreground"
                value={room.boilerplate[k]}
              />
            </div>
          ) : null,
        )}
      </section>

      <section className="flex flex-col gap-6">
        <h2 className="type-h2">{labels.stories}</h2>
        <ul className="grid gap-6 sm:grid-cols-2">
          {room.stories.map((s) => (
            <li key={s.slug}>
              <a
                href={storyHref(s.slug)}
                className="cham group flex h-full flex-col gap-4 p-4 transition-colors hover:[--fill:var(--muted)]"
              >
                {s.cover ? (
                  <div className="cham-sm aspect-[16/9] overflow-hidden">
                    {/* eslint-disable-next-line @next/next/no-img-element */}
                    <img
                      src={mediaUrl(s.cover)}
                      alt={s.coverAlt}
                      loading="lazy"
                      className="relative z-0 h-full w-full object-cover"
                    />
                  </div>
                ) : null}
                <div className="flex flex-col gap-2 px-1 pb-1">
                  <p className="type-mono flex flex-wrap gap-x-3 text-muted-foreground">
                    {s.angle ? <span>{angleLabel(s.angle)}</span> : null}
                    {s.date ? <time dateTime={s.date}>{formatDate(s.date)}</time> : null}
                  </p>
                  <h3 className="type-h3 underline-offset-4 group-hover:underline">{s.title}</h3>
                  {s.summary ? (
                    <p className="type-body text-muted-foreground">{s.summary}</p>
                  ) : null}
                </div>
              </a>
            </li>
          ))}
        </ul>
      </section>

      <section className="flex flex-col gap-3">
        <h2 className="type-h2">{labels.contact}</h2>
        <p className="type-body">
          <a
            href={`mailto:${press.pressEmail}`}
            className="type-mono underline-offset-4 hover:underline"
          >
            {press.pressEmail}
          </a>
        </p>
        {press.spokesperson ? (
          <p className="type-body text-muted-foreground">{press.spokesperson}</p>
        ) : null}
      </section>
    </div>
  );
}
