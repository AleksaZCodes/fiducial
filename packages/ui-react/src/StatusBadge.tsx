import type { Result } from "@fiducial/headless";
import type { CSSProperties } from "react";

export interface StatusBadgeProps<T, E> {
  result: Result<T, E>;
  renderOk?: (value: T) => string;
  renderErr?: (error: E) => string;
}

const baseStyle: CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: "0.375rem",
  padding: "0.25rem 0.75rem",
  borderRadius: "9999px",
  fontSize: "0.75rem",
  fontWeight: 500,
};

export function StatusBadge<T, E>({
  result,
  renderOk = String,
  renderErr = String,
}: StatusBadgeProps<T, E>) {
  if (result.ok) {
    return (
      <span
        style={{
          ...baseStyle,
          background: "var(--secondary)",
          color: "var(--secondary-foreground)",
        }}
      >
        ✓ {renderOk(result.value)}
      </span>
    );
  }
  return (
    <span
      style={{
        ...baseStyle,
        background: "color-mix(in oklch, var(--destructive) 15%, transparent)",
        color: "var(--destructive)",
      }}
    >
      ✗ {renderErr(result.error)}
    </span>
  );
}
