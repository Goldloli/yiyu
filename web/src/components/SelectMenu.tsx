import { Check, ChevronDown } from "lucide-react";
import {
  useEffect,
  useId,
  useRef,
  useState,
  type ReactNode
} from "react";

export interface SelectOption<T extends string> {
  value: T;
  label: string;
  description?: string;
  icon?: ReactNode;
}

export function SelectMenu<T extends string>({
  value,
  options,
  onChange,
  label,
  compact = false,
  disabled = false,
  className = ""
}: {
  value: T;
  options: SelectOption<T>[];
  onChange: (value: T) => void;
  label?: string;
  compact?: boolean;
  disabled?: boolean;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const menuId = useId();
  const selected = options.find((option) => option.value === value);

  useEffect(() => {
    if (!open) return;
    const close = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener("pointerdown", close);
    return () => window.removeEventListener("pointerdown", close);
  }, [open]);

  return (
    <div
      ref={root}
      className={`relative ${className}`}
      onKeyDown={(event) => {
        if (event.key === "Escape" && open) {
          event.preventDefault();
          setOpen(false);
          root.current?.querySelector<HTMLButtonElement>("button")?.focus();
          return;
        }
        if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
        event.preventDefault();
        if (!open) {
          setOpen(true);
          requestAnimationFrame(() => root.current?.querySelector<HTMLButtonElement>('[role="option"][aria-selected="true"]')?.focus());
          return;
        }
        const items = Array.from(root.current?.querySelectorAll<HTMLButtonElement>('[role="option"]') ?? []);
        if (!items.length) return;
        const index = items.indexOf(document.activeElement as HTMLButtonElement);
        const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 :
          event.key === "ArrowDown" ? Math.min(index + 1, items.length - 1) : Math.max(index < 0 ? items.length - 1 : index - 1, 0);
        items[next]?.focus();
      }}
    >
      <button
        type="button"
        className={`control flex w-full items-center gap-2 ${
          compact ? "!min-h-[28px] px-2.5 text-xs" : "px-3"
        }`}
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={menuId}
        disabled={disabled}
        onClick={() => setOpen((value) => !value)}
      >
        {selected?.icon}
        <span className="min-w-0 flex-1 truncate text-left">
          {selected?.label ?? value}
        </span>
        <ChevronDown
          size={compact ? 13 : 15}
          className={`text-tertiary transition-transform ${open ? "rotate-180" : ""}`}
        />
      </button>
      {open && (
        <div
          id={menuId}
          role="listbox"
          className="sheet fade-in absolute left-0 right-0 z-50 mt-1 max-h-72 overflow-auto p-1"
          style={{ minWidth: compact ? 190 : 220 }}
        >
          {options.map((option) => (
            <button
              type="button"
              role="option"
              aria-selected={option.value === value}
              key={option.value}
              className="flex w-full items-center gap-2 rounded-[5px] px-2.5 py-2 text-left hover:bg-[var(--bg-hover)]"
              onClick={() => {
                onChange(option.value);
                setOpen(false);
                requestAnimationFrame(() => root.current?.querySelector<HTMLButtonElement>("button")?.focus());
              }}
            >
              {option.icon}
              <span className="min-w-0 flex-1">
                <span className="block truncate">{option.label}</span>
                {option.description && (
                  <span className="mt-0.5 block truncate text-[10.5px] text-tertiary">
                    {option.description}
                  </span>
                )}
              </span>
              {option.value === value && (
                <Check size={14} className="shrink-0 text-accent" />
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
