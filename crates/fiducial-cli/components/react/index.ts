// Fiducial component registry — React (shadcn conventions).
// Copy-in via `fid add component <name>` — not an installed runtime dependency.
//
// Simple components (button, card, badge) use native HTML + Tailwind + CVA.
// Complex components (dialog) use @base-ui-components/react internally for
// accessible focus trap, keyboard nav, and ARIA — the external API matches shadcn.
//
// Requires in the consuming app: clsx, tailwind-merge, class-variance-authority,
// @base-ui-components/react (dialog only).

export { Button, buttonVariants } from "./button.js";
export type { ButtonProps } from "./button.js";
export { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from "./card.js";
export { Badge, badgeVariants } from "./badge.js";
export {
  Arrow,
  Circle,
  Underline,
  Bracket,
  Burst,
  Check,
  Cross,
  Note,
} from "./doodle.js";
export type { MarkProps, MarkTone, ArrowVariant, UnderlineVariant } from "./doodle.js";
export type { BadgeProps } from "./badge.js";
export {
  Dialog,
  DialogPortal,
  DialogOverlay,
  DialogClose,
  DialogTrigger,
  DialogContent,
  DialogHeader,
  DialogFooter,
  DialogTitle,
  DialogDescription,
} from "./dialog.js";
export {
  Section,
  Band,
  Eyebrow,
  SectionHead,
  Hero,
  LiveDot,
  StepList,
  StatusCard,
  CardGrid,
  Feature,
  FeatureList,
  Cta,
} from "./sections.js";
export type {
  SectionProps,
  BandProps,
  SectionHeadProps,
  HeroProps,
  Step,
  StatusCardProps,
  CtaProps,
} from "./sections.js";
