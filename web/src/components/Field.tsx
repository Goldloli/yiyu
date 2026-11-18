import type { ReactNode } from "react";

export function Field({
  label,
  hint,
  error,
  children
}: {
  label: string;
  hint?: ReactNode;
  error?: string | null;
  children: ReactNode;
}) {
  return (
    <label className="block">
      <span className="mb-1.5 flex items-center justify-between gap-3 text-[12px] font-semibold">
        <span>{label}</span>
        {hint && (
          <span className="text-[10.5px] font-normal text-tertiary">{hint}</span>
        )}
      </span>
      {children}
      {error && <span className="mt-1 block text-[11px] text-danger">{error}</span>}
    </label>
  );
}
