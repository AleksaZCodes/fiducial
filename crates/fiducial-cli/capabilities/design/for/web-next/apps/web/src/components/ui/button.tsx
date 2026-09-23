import { cn } from "@/lib/utils";
import { type VariantProps, cva } from "class-variance-authority";
import { Slot } from "radix-ui";
import type * as React from "react";

/**
 * Button — shadcn/ui's, with the corner and the surface swapped.
 *
 * ## What came from upstream and must not be thrown away
 *
 * This file started as `npx shadcn@latest add button` and the structure is
 * still theirs. The parts that look like boilerplate are the parts that are
 * load-bearing, so they are listed here rather than left to be noticed:
 *
 *   · `focus-visible:ring-[3px]` + `focus-visible:border-ring` — a focus ring
 *     that is actually visible, on keyboard focus only.
 *   · `aria-invalid:` styling — a button that fails validation looks like it.
 *   · `disabled:pointer-events-none disabled:opacity-50` — disabled is
 *     unclickable, not just faded.
 *   · `[&_svg]:pointer-events-none` — an icon inside the button never eats the
 *     click meant for the button.
 *   · `[&_svg:not([class*='size-'])]:size-4` — icons are sized by the button,
 *     so no call site passes `size-4` by hand and gets it wrong.
 *   · `has-[>svg]:px-3` — an icon-bearing button gets tighter padding, because
 *     an icon is optically wider than the space a letter would take.
 *   · `asChild` / `Slot.Root` — the reason a link can be a button without a
 *     `<button>` wrapped in an `<a>`, which is invalid HTML and a real
 *     screen-reader bug.
 *
 * Writing this component from scratch would have meant re-deriving all of that
 * and getting some of it wrong. That is the whole argument for starting from
 * the real thing.
 *
 * ## What FON changed, and why it had to change
 *
 * **The corner.** `rounded-md` is gone. This product's corner strategy is
 * `chamfer` (`design-system.md` § 4), and a chamfer cannot be a border-radius:
 * it is a clip plus a fill, because `clip-path` cuts a real border off along
 * the diagonal and leaves the corner bare. So the shape comes from `.cham-sm`
 * in `marks.css`.
 *
 * **The surface.** Upstream paints the button with `bg-primary`. Here the
 * element's own background *is* the edge, and an inset pseudo-element paints
 * the fill — that is how a chamfered box gets a real 2px border. So every
 * variant sets `--edge` and `--fill` instead of a `bg-*` utility. Setting
 * `bg-primary` on one of these would paint over the edge and flatten the
 * border to nothing, which is the one mistake this file exists to prevent.
 *
 * **The shadow.** `shadow-xs` is gone from `outline`. Not a preference: a
 * `clip-path` element cannot cast an outer `box-shadow` — the clip cuts it
 * off. Depth is carried by the weighted bottom/right edge instead, which every
 * `.cham-*` surface gets by default.
 *
 * **Ghost and link keep their corner square** by simply not being chamfered.
 * A ghost button has no surface to cut.
 */
const buttonVariants = cva(
  [
    "inline-flex shrink-0 items-center justify-center gap-2 whitespace-nowrap",
    "font-display text-sm font-medium transition-all outline-none",
    "focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
    "disabled:pointer-events-none disabled:opacity-50",
    "aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
    "[&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  ].join(" "),
  {
    variants: {
      variant: {
        default:
          "cham-sm text-primary-foreground [--edge:var(--primary)] [--fill:var(--primary)] hover:[--fill:color-mix(in_oklch,var(--primary),black_10%)]",
        destructive:
          "cham-sm text-white [--edge:var(--destructive)] [--fill:var(--destructive)] hover:[--fill:color-mix(in_oklch,var(--destructive),black_10%)] focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40",
        outline: "cham-sm cham-outline",
        secondary:
          "cham-sm text-secondary-foreground [--edge:var(--border)] [--fill:var(--secondary)] hover:[--fill:color-mix(in_oklch,var(--secondary),black_4%)]",
        ghost: "hover:bg-muted hover:text-foreground",
        link: "text-primary underline-offset-4 hover:underline",
      },
      size: {
        // Heights are upstream's. The floor matters here in a way it does not
        // upstream: a chamfer has to stay under half the element's height or
        // the two cuts on one edge meet and the box degenerates into a
        // lozenge. `--corner-control` is 0.5rem, so anything from h-8 up is
        // safe, which is every size below.
        default: "h-9 px-4 py-2 has-[>svg]:px-3",
        sm: "h-8 gap-1.5 px-3 has-[>svg]:px-2.5",
        lg: "h-11 px-6 has-[>svg]:px-4",
        icon: "size-9",
        "icon-sm": "size-8",
        "icon-lg": "size-11",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

function Button({
  className,
  variant = "default",
  size = "default",
  asChild = false,
  ...props
}: React.ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & {
    asChild?: boolean;
  }) {
  const Comp = asChild ? Slot.Root : "button";

  return (
    <Comp
      data-slot="button"
      data-variant={variant}
      data-size={size}
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  );
}

export { Button, buttonVariants };
