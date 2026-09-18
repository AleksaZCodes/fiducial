import { cn } from "@/lib/utils";
import type * as React from "react";

/**
 * Hand-drawn annotation marks — arrows, circles, underlines, brackets.
 *
 * ## What these are for
 *
 * A generated page is uniform: every element carries the same weight, drawn by
 * the same hand, at the same confidence. Annotation breaks that. A mark says
 * *someone looked at this and pointed* — which is the one thing a layout
 * algorithm cannot fake, and the reason the marks are here rather than another
 * icon set.
 *
 * So they are deliberately not icons:
 *   - They overshoot. A circle that closes exactly is a shape; one that
 *     overshoots its start is a gesture.
 *   - They are `aria-hidden` by default and take no space in the accessibility
 *     tree. A mark that points at text is not information, it is emphasis —
 *     the text it points at has to carry the meaning on its own.
 *   - They are stroked in `--doodle-ink`, never `--foreground`. An annotation
 *     at full text contrast is not an annotation, it is a second headline.
 *
 * ## Budget
 *
 * **At most two marks per viewport, and at most one of them `tone="accent"`.**
 * This is the rule that keeps the device from becoming the slop it replaced.
 * Three arrows on one screen is a clip-art page.
 *
 * ## Requires
 *
 * The marks CSS layer (`design-system/marks.css` from the `design` capability),
 * which defines `.doodle` and the draw animation. Without it the marks still
 * render — they just appear fully drawn, with no stroke animation, which is
 * also exactly what happens under `prefers-reduced-motion`.
 *
 * ## Drawing
 *
 * `pathLength={1}` on every path is what makes one animation work for all of
 * them: it renormalises each path to a length of 1 regardless of its real
 * geometry, so `stroke-dasharray: 1; stroke-dashoffset: 1 → 0` draws any mark
 * in the same time. Without it, the long arrow and the short check would draw
 * at wildly different speeds from the same rule.
 */

// ── Shared primitive ─────────────────────────────────────────────────────────

/** Which ink a mark is stroked in. */
export type MarkTone =
  /** The quiet one. Default, and what nearly every mark should be. */
  | "ink"
  /** The loud one. At most one per viewport. */
  | "accent"
  /** Inherits `color` — for marks sitting inside already-coloured text. */
  | "current";

export interface MarkProps extends Omit<React.SVGProps<SVGSVGElement>, "ref"> {
  tone?: MarkTone;
  /**
   * Mirror horizontally. An arrow drawn pointing right, pointing left instead —
   * so the set does not need a second copy of every path.
   */
  flip?: boolean;
  /** Rotation in degrees, applied after `flip`. */
  rotate?: number;
  /** Animate the stroke on. Default true; ignored under prefers-reduced-motion. */
  draw?: boolean;
  /**
   * Give the mark an accessible name, making it visible to assistive tech.
   * Leave unset — the default is decorative, which is almost always correct.
   */
  label?: string;
}

const toneStroke: Record<MarkTone, string> = {
  ink: "var(--doodle-ink)",
  accent: "var(--doodle-accent)",
  current: "currentColor",
};

function Mark({
  tone = "ink",
  flip = false,
  rotate,
  draw = true,
  label,
  className,
  style,
  children,
  ...props
}: MarkProps & { children: React.ReactNode }) {
  const transform =
    [flip ? "scaleX(-1)" : null, rotate ? `rotate(${rotate}deg)` : null]
      .filter(Boolean)
      .join(" ") || undefined;

  return (
    <svg
      // `overflow: visible` because several marks are drawn to overshoot their
      // own viewBox on purpose — clipping them back to the box is what makes a
      // gesture look like a sticker.
      className={cn("doodle", draw && "doodle-draw", className)}
      fill="none"
      stroke={toneStroke[tone]}
      strokeWidth="var(--doodle-stroke, 2.25px)"
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{ overflow: "visible", transform, ...style }}
      role={label ? "img" : undefined}
      aria-label={label}
      aria-hidden={label ? undefined : true}
      {...props}
    >
      {label ? <title>{label}</title> : null}
      {children}
    </svg>
  );
}

/** Every path in the set is renormalised, so one animation fits all of them. */
function P({ d, delay }: { d: string; delay?: number }) {
  return (
    <path
      d={d}
      pathLength={1}
      style={delay ? ({ "--doodle-delay": `${delay}ms` } as React.CSSProperties) : undefined}
    />
  );
}

// ── Arrow ────────────────────────────────────────────────────────────────────

export type ArrowVariant =
  /** A gentle arc. The default, and the one that reads as "over here". */
  | "curve"
  /** Curls back on itself before pointing. For the aside you want read second. */
  | "loop"
  /** A sharp elbow. For pointing around something rather than across it. */
  | "kink"
  /** Almost straight, with just enough wobble to not be a border. */
  | "straight";

const ARROWS: Record<ArrowVariant, { viewBox: string; shaft: string; head: string }> = {
  curve: {
    viewBox: "0 0 120 44",
    shaft: "M4 11 C 32 3, 74 7, 103 27",
    head: "M 87 20 L 105 29 L 94 12",
  },
  loop: {
    viewBox: "0 0 120 70",
    shaft:
      "M5 60 C 17 24, 53 5, 70 21 C 81 32, 62 47, 51 36 C 39 24, 63 9, 89 16 C 101 19, 108 26, 112 35",
    head: "M 103 25 L 113 37 L 98 40",
  },
  kink: {
    viewBox: "0 0 120 56",
    shaft: "M6 9 C 35 5, 52 14, 56 30 C 59 42, 70 47, 103 43",
    head: "M 91 35 L 106 43 L 92 51",
  },
  straight: {
    viewBox: "0 0 120 30",
    shaft: "M5 17 C 31 11, 61 19, 105 14",
    head: "M 93 7 L 107 14 L 94 22",
  },
};

export interface ArrowProps extends MarkProps {
  variant?: ArrowVariant;
}

/** An arrow that points at something. Pair it with a `<Note>`, not with a heading. */
export function Arrow({ variant = "curve", ...props }: ArrowProps) {
  const a = ARROWS[variant];
  return (
    <Mark viewBox={a.viewBox} {...props}>
      <P d={a.shaft} />
      {/* The head is drawn after the shaft arrives, not alongside it. */}
      <P d={a.head} delay={420} />
    </Mark>
  );
}

// ── Circle ───────────────────────────────────────────────────────────────────

export interface CircleProps extends MarkProps {
  /** Draw a second, looser pass around the first — the "I mean it" circle. */
  twice?: boolean;
}

/**
 * A loop scribbled around something. Position it absolutely behind the thing it
 * circles, with the circled element `position: relative; z-index: 1`.
 */
export function Circle({ twice = false, ...props }: CircleProps) {
  return (
    <Mark viewBox="0 0 200 90" {...props}>
      <P d="M150 12 C 109 1, 39 5, 19 30 C 1 52, 30 78, 85 82 C 141 86, 187 69, 188 43 C 189 21, 159 9, 127 8" />
      {twice ? (
        <P
          d="M144 20 C 108 11, 47 14, 29 34 C 14 52, 38 73, 86 76 C 135 79, 178 66, 179 45 C 180 27, 156 17, 130 16"
          delay={300}
        />
      ) : null}
    </Mark>
  );
}

// ── Underline ────────────────────────────────────────────────────────────────

export type UnderlineVariant = "squiggle" | "single" | "double";

const UNDERLINES: Record<UnderlineVariant, { viewBox: string; paths: string[] }> = {
  squiggle: {
    viewBox: "0 0 120 12",
    paths: [
      "M2 7 C 12 2, 20 10, 30 6 C 40 2, 48 10, 58 6 C 68 2, 76 10, 86 6 C 96 2, 106 10, 118 5",
    ],
  },
  single: {
    viewBox: "0 0 120 10",
    paths: ["M3 6 C 34 1, 78 9, 117 4"],
  },
  double: {
    viewBox: "0 0 120 14",
    paths: ["M3 5 C 33 1, 79 8, 117 3", "M6 12 C 38 8, 74 13, 114 9"],
  },
};

export interface UnderlineProps extends MarkProps {
  variant?: UnderlineVariant;
}

/**
 * Underline a phrase. Stretch it with `w-full` under an inline-block span —
 * `preserveAspectRatio="none"` is deliberately NOT set, because a squiggle
 * stretched to four times its width stops looking drawn.
 */
export function Underline({ variant = "squiggle", ...props }: UnderlineProps) {
  const u = UNDERLINES[variant];
  return (
    <Mark viewBox={u.viewBox} {...props}>
      {u.paths.map((d, i) => (
        <P key={d} d={d} delay={i * 260} />
      ))}
    </Mark>
  );
}

// ── Bracket ──────────────────────────────────────────────────────────────────

export interface BracketProps extends MarkProps {
  /** Which side of the content it hugs. */
  side?: "left" | "right";
}

/** A hand-drawn brace beside a block — "all of this, together". */
export function Bracket({ side = "left", flip, ...props }: BracketProps) {
  return (
    <Mark viewBox="0 0 24 120" flip={flip ?? side === "right"} {...props}>
      <P d="M20 3 C 9 9, 6 39, 7 59 C 8 79, 10 111, 20 117" />
      <P d="M7 59 C 5 57, 3 56, 1 55" delay={340} />
    </Mark>
  );
}

// ── Emphasis marks ───────────────────────────────────────────────────────────

/** A spark. For the one thing on the page that is genuinely new. */
export function Burst(props: MarkProps) {
  return (
    <Mark viewBox="0 0 44 44" {...props}>
      <P d="M22 3 C 22 10, 21 15, 21 19" />
      <P d="M41 22 C 34 22, 29 22, 25 22" delay={90} />
      <P d="M22 41 C 22 34, 22 29, 23 25" delay={180} />
      <P d="M3 22 C 10 22, 15 22, 19 22" delay={270} />
      <P d="M35 9 C 31 13, 28 16, 26 18" delay={360} />
      <P d="M35 35 C 31 31, 28 28, 26 26" delay={430} />
      <P d="M9 35 C 13 31, 16 28, 18 26" delay={500} />
      <P d="M9 9 C 13 13, 16 16, 18 18" delay={570} />
    </Mark>
  );
}

/** Confirmed, done, true. */
export function Check(props: MarkProps) {
  return (
    <Mark viewBox="0 0 44 36" {...props}>
      <P d="M3 18 C 9 22, 13 27, 17 33 C 23 20, 31 9, 42 2" />
    </Mark>
  );
}

/** Dismissed, false, not this. */
export function Cross(props: MarkProps) {
  return (
    <Mark viewBox="0 0 36 36" {...props}>
      <P d="M4 4 C 13 13, 22 23, 32 33" />
      <P d="M32 4 C 23 13, 13 23, 4 33" delay={220} />
    </Mark>
  );
}

// ── Note ─────────────────────────────────────────────────────────────────────

export interface NoteProps extends React.HTMLAttributes<HTMLElement> {
  /** Tilt in degrees. Small. A note at 12° is a sticker. */
  tilt?: number;
  tone?: Exclude<MarkTone, "current">;
}

/**
 * The handwriting that goes with a mark — the `note` step of the type scale,
 * in the script role, tilted a couple of degrees off true.
 *
 * It is a `<span>` and not a heading on purpose: an annotation is an aside in
 * the document outline, and promoting it to `<h3>` puts "this is the part
 * everyone argues about" into the table of contents.
 */
export function Note({ tilt = -3, tone = "ink", className, style, ...props }: NoteProps) {
  return (
    <span
      className={cn("doodle-note", className)}
      style={{
        transform: `rotate(${tilt}deg)`,
        color: toneStroke[tone],
        ...style,
      }}
      {...props}
    />
  );
}
