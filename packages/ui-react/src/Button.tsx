import type { CSSProperties, ReactNode } from "react";

export interface ButtonProps {
  children: ReactNode;
  variant?: "primary" | "secondary" | "ghost";
  disabled?: boolean;
  onClick?: () => void;
}

const variantStyles: Record<string, CSSProperties> = {
  primary: {
    background: "var(--primary)",
    color: "var(--primary-foreground)",
    border: "none",
  },
  secondary: {
    background: "var(--secondary)",
    color: "var(--secondary-foreground)",
    border: "1px solid var(--border)",
  },
  ghost: {
    background: "transparent",
    color: "var(--foreground)",
    border: "1px solid transparent",
  },
};

const baseStyle: CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  padding: "0.5rem 1rem",
  borderRadius: "var(--radius)",
  fontSize: "0.875rem",
  fontWeight: 500,
  cursor: "pointer",
  transition: "opacity 0.15s",
};

export function Button({
  children,
  variant = "primary",
  disabled = false,
  onClick,
}: ButtonProps) {
  return (
    <button
      style={{
        ...baseStyle,
        ...variantStyles[variant],
        opacity: disabled ? 0.5 : 1,
        cursor: disabled ? "not-allowed" : "pointer",
      }}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}
