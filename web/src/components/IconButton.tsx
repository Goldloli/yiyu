import type { ButtonHTMLAttributes, ReactNode } from "react";

export function IconButton({
  label,
  active,
  children,
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  label: string;
  active?: boolean;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      className={`icon-button ${className}`}
      aria-label={label}
      title={label}
      data-active={active ? "true" : undefined}
      {...props}
    >
      {children}
    </button>
  );
}
