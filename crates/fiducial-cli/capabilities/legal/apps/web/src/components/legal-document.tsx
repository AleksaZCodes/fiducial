/**
 * Renders one generated legal page.
 *
 * Installed by the `legal` capability. The content comes from
 * `src/generated/legal.ts`, which `fid derive` writes from `[legal]`,
 * `[brand]` and `[i18n]` in `fiducial.toml`. Nothing here supplies copy: this
 * is the presentation half, and a sentence typed into this file is a sentence
 * that exists in one language and is invisible to `fid derive --check`.
 *
 * ## Why there is a Markdown parser in here
 *
 * The generator emits a deliberately tiny subset: `## headings`, `**bold**`
 * runs, `*italic*` lines, and paragraphs separated by blank lines. That is
 * enough for a legal page and small enough to parse in forty lines, which is
 * cheaper than adding a Markdown dependency to every scaffolded product for
 * four constructs. If the generated copy ever needs a list or a table, extend
 * the generator and this together, in one commit.
 *
 * It renders on the server. No `"use client"`, no hydration cost, and the text
 * is in the HTML for a crawler and for someone with JavaScript disabled, which
 * for a privacy policy is closer to a requirement than a nicety.
 */

/**
 * Split `**bold**` runs out of a line, preserving order.
 *
 * Keyed by each run's character offset rather than by its array index. The
 * offset is a real identity — it says *where in this string this run is* — and
 * survives the text changing around it, which an index does not.
 */
function inline(text: string, keyPrefix: string) {
  let offset = 0;
  return text.split(/(\*\*[^*]+\*\*)/g).map((part) => {
    const at = offset;
    offset += part.length;
    if (part.startsWith("**") && part.endsWith("**") && part.length > 4) {
      return (
        <strong key={`${keyPrefix}@${at}`} className="font-semibold text-foreground">
          {part.slice(2, -2)}
        </strong>
      );
    }
    return part;
  });
}

export interface LegalDocumentProps {
  title: string;
  /** The generated body. Markdown, in the subset described above. */
  body: string;
  /** Rendered above the title, e.g. a breadcrumb or the word "Legal". */
  kicker?: string;
}

export function LegalDocument({ title, body, kicker }: LegalDocumentProps) {
  const blocks = body.split(/\n{2,}/).filter((b) => b.trim().length > 0);

  return (
    <article className="mx-auto max-w-[68ch]">
      {kicker ? <p className="type-small text-muted-foreground">{kicker}</p> : null}
      <h1 className="type-h1 mt-2">{title}</h1>

      <div className="mt-10 space-y-6">
        {blocks.map((block, i) => {
          const key = `b${i}`;
          const trimmed = block.trim();

          if (trimmed.startsWith("## ")) {
            return (
              <h2 key={key} className="type-h3 mt-12 first:mt-0">
                {trimmed.slice(3)}
              </h2>
            );
          }

          // A whole-line italic is the generator's "meta" voice: the review
          // date, or the marker saying nobody has reviewed this. It is set
          // apart rather than inline, because a reader deciding whether to
          // trust the page should not have to find it inside a paragraph.
          if (trimmed.startsWith("*") && trimmed.endsWith("*") && !trimmed.startsWith("**")) {
            return (
              <p
                key={key}
                className="type-small border-l-2 border-border pl-4 text-muted-foreground italic"
              >
                {trimmed.slice(1, -1)}
              </p>
            );
          }

          // A block whose lines are all short and unpunctuated is an address
          // or an identifier stack, not prose. Keep its line breaks.
          if (trimmed.includes("\n")) {
            return (
              <p key={key} className="type-body whitespace-pre-line text-muted-foreground">
                {inline(trimmed, key)}
              </p>
            );
          }

          return (
            <p key={key} className="type-body text-muted-foreground">
              {inline(trimmed, key)}
            </p>
          );
        })}
      </div>
    </article>
  );
}
