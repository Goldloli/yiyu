export function SegmentedControl<T extends string>({
  value,
  options,
  onChange,
  label
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  label: string;
}) {
  return (
    <div
      role="group"
      aria-label={label}
      className="inline-flex rounded-[6px] border border-[var(--line-strong)] bg-[var(--bg-input)] p-[2px]"
    >
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          aria-pressed={option.value === value}
          className="rounded-[4px] px-2.5 py-1 text-[11px] font-medium transition-colors"
          style={{
            color: option.value === value ? "var(--text)" : "var(--text-secondary)",
            background: option.value === value ? "var(--selected-surface)" : "transparent"
          }}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
