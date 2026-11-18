import type { ButtonHTMLAttributes, ReactNode } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";

export function Button({
  variant = "secondary",
  icon,
  children,
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: Variant;
  icon?: ReactNode;
}) {
  const classes =
    variant === "primary"
      ? "button-primary"
      : variant === "ghost"
        ? "button-ghost"
        : variant === "danger"
          ? "button-secondary button-danger"
          : "button-secondary";
  return (
    <button type="button" className={`${classes} ${className}`} {...props}>
      {icon}
      {children}
    </button>
  );
}
