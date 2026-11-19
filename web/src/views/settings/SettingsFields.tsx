import type { ReactNode } from "react";

export function PageHeading({
  title,
  action
}: {
  title: string;
  action?: ReactNode;
}) {
  return (
    <div className="mb-5 flex items-start gap-4">
      <div className="min-w-0 flex-1">
        <h1 className="text-[20px] font-semibold tracking-[-0.01em]">{title}</h1>
      </div>
      {action}
    </div>
  );
}

export function PropertyRow({
  title,
  description,
  control
}: {
  title: string;
  description?: string;
  control: ReactNode;
}) {
  return (
    <div className="property-row">
      <div className="min-w-0">
        <p className="text-[13px] font-medium">{title}</p>
        {description && (
          <p className="mt-0.5 text-[13px] leading-4 text-tertiary">{description}</p>
        )}
      </div>
      <div className="shrink-0">{control}</div>
    </div>
  );
}
