import { mergeProps } from "@base-ui/react/merge-props"
import { useRender } from "@base-ui/react/use-render"
import type * as React from "react"
import { cn } from "@/lib/utils"

/**
 * Card — shadcn/ui's, with the corner and the surface swapped.
 *
 * Upstream's own classes are kept wherever they are doing structural work:
 * the `@container/card-header` query, the `has-data-[slot=card-action]`
 * grid switch, the `[.border-b]:pb-6` divider padding. Those are the parts
 * that make `CardHeader` behave when an action button is dropped into it, and
 * re-deriving them by hand is how a card ends up with its title and its button
 * fighting over one row.
 *
 * Three things changed, for the same reasons the Button changed:
 *
 *   · `rounded-xl` → `.cham`. The corner strategy is `chamfer`, and a chamfer
 *     is a clip plus a fill rather than a radius.
 *   · `border bg-card` → `--edge` / `--fill`. On a chamfered box the element's
 *     own background paints the edge and an inset pseudo-element paints the
 *     fill, so a `bg-*` utility here would paint over the border entirely.
 *   · `shadow-sm` is gone. A `clip-path` element cannot cast an outer shadow —
 *     the clip removes it. Depth comes from the weighted bottom/right edge and
 *     the inset top highlight instead, which every `.cham` surface gets for
 *     free. This is the one place where "no shadows" is physics rather than
 *     taste, and it is why it is written down in three files.
 */
/**
 * `render` is an addition, not an adaptation.
 *
 * Upstream's Card is always a `<div>`. A card inside an `<ol>` or a `<ul>` has
 * to be an `<li>` — a `<div>` between a list and its items breaks the list in
 * the accessibility tree, and a screen reader stops announcing "4 items". So
 * Card takes a `render` prop, the same composition Button has:
 * `<Card render={<li />}>`. The list markup on the landing page is the reason
 * this exists.
 *
 * It was `asChild` through a Radix `Slot`; Fiducial's components sit on Base
 * UI, whose `useRender` is the equivalent.
 */
function Card({
  className,
  render,
  ...props
}: useRender.ComponentProps<"div">) {
  return useRender({
    defaultTagName: "div",
    render,
    props: mergeProps<"div">(
      {
        className: cn(
          "cham flex flex-col gap-6 py-6 text-card-foreground",
          className
        ),
      },
      props
    ),
    // `useRender` writes state as data attributes: this is `data-slot="card"`,
    // the same way shadcn's Base UI badge sets its slot.
    state: { slot: "card" },
  })
}

function CardHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-header"
      className={cn(
        "@container/card-header grid auto-rows-min grid-rows-[auto_auto] items-start gap-2 px-6 has-data-[slot=card-action]:grid-cols-[1fr_auto] [.border-b]:pb-6",
        className
      )}
      {...props}
    />
  )
}

function CardTitle({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-title"
      className={cn("leading-none font-semibold", className)}
      {...props}
    />
  )
}

function CardDescription({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-description"
      className={cn("text-sm text-muted-foreground", className)}
      {...props}
    />
  )
}

function CardAction({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-action"
      className={cn(
        "col-start-2 row-span-2 row-start-1 self-start justify-self-end",
        className
      )}
      {...props}
    />
  )
}

function CardContent({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-content"
      className={cn("px-6", className)}
      {...props}
    />
  )
}

function CardFooter({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-footer"
      className={cn("flex items-center px-6 [.border-t]:pt-6", className)}
      {...props}
    />
  )
}

export {
  Card,
  CardHeader,
  CardFooter,
  CardTitle,
  CardAction,
  CardDescription,
  CardContent,
}
