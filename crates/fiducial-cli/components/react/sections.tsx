import type * as React from "react";
import { cn } from "@/lib/utils";

/**
 * Landing-page sections.
 *
 * ## Why these exist
 *
 * A component library gives you a button and leaves the page to you — which is
 * exactly where a design system stops being applied. The page is where the
 * defaults win: full-bleed gradient hero, icon grid, testimonial carousel,
 * three-column footer. Not because anyone chose them, but because nobody
 * wrote down what to do instead.
 *
 * So the sections are components too. Each one is a *shape* the design system
 * has already decided — rhythm, measure, which type step, where the rule goes —
 * and every one of them reads named steps and tokens rather than literals. A
 * page assembled from these is consistent by construction, and a page that
 * needs something else should say so in `design-system.md` first.
 *
 * ## What they deliberately do not do
 *
 * No gradients, no shadows, no hover lifts, no scroll-reveal, no carousels, no
 * icon grids, no `rounded-full`. Those are on the "not this" list, and a
 * component that offers them as a prop is a list that does not hold.
 */

// ── Rhythm ───────────────────────────────────────────────────────────────────

export interface SectionProps extends React.HTMLAttributes<HTMLElement> {
  /** Tighten the vertical rhythm. For a band that is a single statement. */
  tight?: boolean;
  as?: "section" | "div" | "footer" | "header";
}

/**
 * The page's horizontal measure and vertical rhythm, in one place.
 *
 * Every section on a page uses this, so the gutters cannot drift apart between
 * two screens written a week apart.
 */
export function Section({ tight, as: Tag = "section", className, ...props }: SectionProps) {
  return (
    <Tag
      className={cn(
        "mx-auto w-full max-w-7xl px-5 sm:px-8",
        tight ? "py-16 sm:py-20" : "py-20 sm:py-28",
        className,
      )}
      {...props}
    />
  );
}

export interface BandProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Rule above, below, or both. Bands are separated by a line, not by air. */
  rule?: "top" | "bottom" | "both" | "none";
  /** Tint the band with `--muted`, for a section that should sit back. */
  tinted?: boolean;
}

/**
 * A full-bleed horizontal band behind a `<Section>`.
 *
 * Sections are separated by a rule rather than by whitespace alone — whitespace
 * is how a page ends up 4000px tall with six ideas in it, and a reader skimming
 * for structure has nothing to catch on.
 */
export function Band({ rule = "both", tinted = true, className, ...props }: BandProps) {
  return (
    <div
      className={cn(
        tinted && "bg-muted/40",
        rule === "top" && "border-t border-border",
        rule === "bottom" && "border-b border-border",
        rule === "both" && "border-y border-border",
        className,
      )}
      {...props}
    />
  );
}

/**
 * `Omit<…, "title">` throughout: every HTML element already has a `title`
 * attribute, and it is the tooltip string. These components use `title` for a
 * heading, which is a `ReactNode`. Omitting is the honest fix — widening the
 * attribute would let an element be passed through to `title=""`, where React
 * renders it as `[object Object]` in a tooltip.
 */

// ── Headings ─────────────────────────────────────────────────────────────────

/**
 * The label above a section heading — display face, tracked out, in `--primary`.
 *
 * It is a `<p>` and not a heading: an eyebrow is a label for the heading below
 * it, and promoting it to `<h2>` puts "PRINCIPLE" into the document outline
 * immediately above the thing it is labelling.
 */
export function Eyebrow({ className, ...props }: React.HTMLAttributes<HTMLParagraphElement>) {
  return <p className={cn("type-eyebrow mb-3 text-primary", className)} {...props} />;
}

export interface SectionHeadProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "title"> {
  eyebrow?: React.ReactNode;
  title: React.ReactNode;
  /** The sentence under the heading. Body face, larger, muted. */
  lede?: React.ReactNode;
  /** `h2` by default; `h1` only for the one section that is the page's subject. */
  level?: "h1" | "h2";
}

/** Eyebrow, heading and lede, at the measures the scale specifies. */
export function SectionHead({
  eyebrow,
  title,
  lede,
  level = "h2",
  className,
  ...props
}: SectionHeadProps) {
  const Heading = level;
  return (
    <div className={className} {...props}>
      {eyebrow ? <Eyebrow>{eyebrow}</Eyebrow> : null}
      {/* Headings cap at 28 characters of measure, not at a pixel width: a
          display line is sized by how many words the eye takes in at once. */}
      <Heading className={cn(level === "h1" ? "type-h1" : "type-h2", "max-w-[28ch]")}>
        {title}
      </Heading>
      {lede ? <p className="type-subhead mt-4 max-w-[62ch] text-muted-foreground">{lede}</p> : null}
    </div>
  );
}

// ── Hero ─────────────────────────────────────────────────────────────────────

export interface HeroProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "title"> {
  /** A short status pill above the headline — "In development", "Beta". */
  status?: React.ReactNode;
  title: React.ReactNode;
  subtitle?: React.ReactNode;
  /** Buttons. One primary, and that is the whole budget. */
  actions?: React.ReactNode;
  /** A qualification under the fold of the hero, set off by a left rule. */
  footnote?: React.ReactNode;
}

/**
 * The hero.
 *
 * Left-aligned in a column, not centred over a full-bleed gradient. Centred
 * hero text is the single most recognisable generated-page shape there is, and
 * it reads worse: a centred block has no consistent left edge for the eye to
 * return to, so every line starts somewhere new.
 *
 * There is no `image` or `background` prop. If this hero needs a backdrop, put
 * it on the element around it — and say in `design-system.md` what it is.
 */
export function Hero({
  status,
  title,
  subtitle,
  actions,
  footnote,
  className,
  ...props
}: HeroProps) {
  return (
    <div className={cn("max-w-3xl", className)} {...props}>
      {status ? (
        <span className="cham-xs type-small inline-flex items-center gap-2 px-3.5 py-2 text-muted-foreground">
          {status}
        </span>
      ) : null}

      <h1 className={cn("type-display", status && "mt-6")}>{title}</h1>

      {subtitle ? (
        <p className="type-subhead mt-6 max-w-[62ch] text-muted-foreground">{subtitle}</p>
      ) : null}

      {actions ? <div className="mt-9 flex flex-wrap items-center gap-3">{actions}</div> : null}

      {footnote ? (
        <p className="type-small mt-8 max-w-[58ch] border-l-2 border-border pl-4 text-muted-foreground">
          {footnote}
        </p>
      ) : null}
    </div>
  );
}

/** A small pulsing dot, for a status pill that is reporting something live. */
export function LiveDot({ className }: { className?: string }) {
  return (
    <span className={cn("relative flex h-1.5 w-1.5", className)} aria-hidden="true">
      <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-accent opacity-60" />
      <span className="relative inline-flex h-1.5 w-1.5 rounded-full bg-accent" />
    </span>
  );
}

// ── Sequences and grids ──────────────────────────────────────────────────────

export interface Step {
  /** Rendered in the mono face. Two digits reads better than one. */
  n: string;
  title: React.ReactNode;
  body: React.ReactNode;
  /** An annotation to hang on this step. At most one per page. */
  aside?: React.ReactNode;
}

/**
 * A numbered process, as an ordered list of cards.
 *
 * `<ol>`, because it is a sequence and the order is the content — a screen
 * reader should say "3 of 4", and a `<div>` grid cannot.
 *
 * Two columns rather than four: four across turns any step longer than a line
 * into a column of six-word rows, and the shape starts reading as a feature
 * grid rather than as a process.
 */
export function StepList({ steps, className }: { steps: Step[]; className?: string }) {
  return (
    <ol className={cn("grid gap-4 sm:grid-cols-2", className)}>
      {steps.map((s) => (
        <li key={s.n} className="cham p-6">
          <span className="type-mono text-primary">{s.n}</span>
          <h3 className="type-h3 mt-3">{s.title}</h3>
          <p className="type-small mt-2 max-w-[52ch] text-muted-foreground">{s.body}</p>
          {s.aside}
        </li>
      ))}
    </ol>
  );
}

export interface StatusCardProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "title"> {
  title: React.ReactNode;
  /**
   * What is actually true about this thing — "Designed", "Planned", "Shipped".
   *
   * Load-bearing, and the first place a product starts telling comfortable
   * lies. If the label is doing no work, do not render one.
   */
  status?: React.ReactNode;
}

/** A card with an honest label in its corner. Flat: no shadow, no hover lift. */
export function StatusCard({ title, status, children, className, ...props }: StatusCardProps) {
  return (
    <div className={cn("cham p-6", className)} {...props}>
      <div className="flex items-start justify-between gap-4">
        <h3 className="type-h3">{title}</h3>
        {status ? (
          <span className="cham-xs type-small shrink-0 px-3 py-1.5 text-[0.7rem] font-medium uppercase tracking-wide text-muted-foreground">
            {status}
          </span>
        ) : null}
      </div>
      <div className="type-small mt-3 max-w-[52ch] text-muted-foreground">{children}</div>
    </div>
  );
}

/** A plain two-column grid at the standard gutter. */
export function CardGrid({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("grid gap-4 sm:grid-cols-2", className)} {...props} />;
}

/**
 * A point made with a rule beside it rather than a card around it.
 *
 * This is what replaces the icon grid. An icon beside a four-word label is
 * decoration standing in for a reason; a rule and a real sentence is the
 * thing itself.
 */
export function Feature({
  title,
  children,
  className,
  ...props
}: { title: React.ReactNode } & Omit<React.HTMLAttributes<HTMLDivElement>, "title">) {
  return (
    <div className={cn("border-l-2 border-primary/40 pl-5", className)} {...props}>
      <h3 className="type-h3">{title}</h3>
      <div className="type-small mt-2 max-w-[52ch] text-muted-foreground">{children}</div>
    </div>
  );
}

/** Features in a two-column grid, with room between them. */
export function FeatureList({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("grid gap-8 sm:grid-cols-2", className)} {...props} />;
}

// ── Close ────────────────────────────────────────────────────────────────────

export interface CtaProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "title"> {
  eyebrow?: React.ReactNode;
  title: React.ReactNode;
  body?: React.ReactNode;
  action?: React.ReactNode;
  /** An address, a repository, the thing they will actually copy. */
  detail?: React.ReactNode;
}

/** The closing ask. One action, narrow measure, nothing competing with it. */
export function Cta({ eyebrow, title, body, action, detail, className, ...props }: CtaProps) {
  return (
    <div className={cn("max-w-2xl", className)} {...props}>
      {eyebrow ? <Eyebrow>{eyebrow}</Eyebrow> : null}
      <h2 className="type-h2">{title}</h2>
      {body ? <p className="type-body mt-5 max-w-[68ch] text-muted-foreground">{body}</p> : null}
      {action ? <div className="mt-8">{action}</div> : null}
      {detail ? <p className="type-mono mt-4 text-muted-foreground">{detail}</p> : null}
    </div>
  );
}
