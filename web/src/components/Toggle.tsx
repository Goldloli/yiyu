export function Toggle({
  checked,
  onChange,
  label,
  disabled = false
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className="relative h-[20px] w-[34px] shrink-0 rounded-full border transition-colors disabled:opacity-50"
      style={{
        borderColor: checked ? "var(--accent)" : "var(--line-strong)",
        background: checked ? "var(--accent)" : "var(--bg-input)"
      }}
    >
      <span
        className="absolute top-[3px] h-[12px] w-[12px] rounded-full bg-white shadow-sm transition-[left]"
        style={{ left: checked ? 18 : 3 }}
      />
    </button>
  );
}
