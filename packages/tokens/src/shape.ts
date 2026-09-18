/**
 * Shape tokens — the corner strategy, the border weight, and the annotation ink.
 *
 * ## Why a "corner strategy" and not just `--radius`
 *
 * The shadcn slot set has one shape decision in it: `--radius`. That is why
 * every product built on it has the same silhouette — a 0.5rem rounded
 * rectangle — and why "large rounded corners on cards and buttons" is on the
 * anti-slop list. One variable can only say *how much*, never *what kind*.
 *
 * So shape is declared as a strategy plus a three-step scale:
 *
 *   strategy   `round` | `chamfer` | `square`
 *   panel      cards, dialogs, sections        (elements taller than ~5rem)
 *   control    buttons, inputs, nav items      (~2.25rem and up)
 *   chip       badges, pills, tags             (~1.75rem and up)
 *
 * Three steps rather than Tailwind's six, because a corner size that is not
 * tied to an element class is a corner size that drifts. And three rather than
 * one, because a chamfer has a hard geometric constraint a radius does not:
 *
 *   **A chamfer must stay under half the element's height.** Past that the two
 *   cuts on one edge meet and the box degenerates into a lozenge. This is why
 *   `chip` exists as its own step and why `panel` must never be applied to a
 *   badge.
 *
 * `--radius` is still emitted, because the shadcn slot contract references it
 * and a component copied in from any registry expects it to resolve. Under the
 * `chamfer` and `square` strategies it is `0rem` and the corner comes from the
 * `.cham-*` classes in the marks layer instead.
 */

/** How corners are cut. Declared once per product; components never override it. */
export type CornerStrategy =
  /** Rounded. `--radius` carries the value and `.cham-*` are no-ops. */
  | 'round'
  /** Cut at 45°. `--radius` is 0 and `.cham-*` carry the shape. */
  | 'chamfer'
  /** No corner treatment at all. Harder than it looks to pull off; commit or don't. */
  | 'square'

/** The three corner sizes, as CSS custom property references. */
export const corners = {
  panel: 'var(--corner-panel)',
  control: 'var(--corner-control)',
  chip: 'var(--corner-chip)',
} as const

/** Border weight. One value: a product with three border weights has none. */
export const strokes = {
  /** Structural edges — card outlines, the nav bar, chamfer edges. */
  edge: 'var(--border-w)',
  /** Hairlines — table rules, dividers, the things you should barely see. */
  hair: 'var(--hair-w)',
} as const

/**
 * The ink the hand-drawn marks are stroked in.
 *
 * Separate from `--foreground` on purpose: annotation is a second voice, and a
 * second voice at full text contrast is not an annotation, it is a second
 * headline. `--doodle-ink` is deliberately quieter than body text, and
 * `--doodle-accent` is the one that gets to shout — used for at most one mark
 * per viewport.
 */
export const doodle = {
  ink: 'var(--doodle-ink)',
  accent: 'var(--doodle-accent)',
  /** Stroke width of a mark, in px. Marks are drawn at a fixed weight, not scaled. */
  stroke: 'var(--doodle-stroke)',
} as const

/** Shape values Fiducial ships. Products override the whole object, not one key. */
export const shapeDefaults = {
  strategy: 'chamfer' as CornerStrategy,
  panel: '1.5rem',
  control: '0.75rem',
  chip: '0.5rem',
  edge: '2px',
  hair: '1px',
  doodleStroke: '2.25px',
} as const

export type Corners = typeof corners
export type Strokes = typeof strokes
export type Doodle = typeof doodle
export type ShapeDefaults = typeof shapeDefaults
