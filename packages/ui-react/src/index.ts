// Fiducial component registry — React.
// Copy-in via `fid add component <name>` — not an installed runtime dependency.
//
// Available:
//   button  Button (primary | secondary | ghost | destructive)
//   card    Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter
//   badge   Badge (default | secondary | destructive | outline)
//   dialog  Dialog (Base UI — accessible focus trap, keyboard nav, ARIA)

export { Button } from "./button.js";
export type { ButtonProps } from "./button.js";
export { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from "./card.js";
export { Badge } from "./badge.js";
export type { BadgeProps } from "./badge.js";
export {
  Dialog,
  DialogTrigger,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogClose,
  DialogFooter,
} from "./dialog.js";
export type { DialogProps } from "./dialog.js";
