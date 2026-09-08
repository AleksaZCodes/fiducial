import * as React from "react";

export interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  variant?: "default" | "secondary" | "destructive" | "outline";
}

export function Badge({ variant = "default", className, ...props }: BadgeProps) {
  return (
    <span
      data-variant={variant}
      className={["fid-badge", className].filter(Boolean).join(" ")}
      {...props}
    />
  );
}
