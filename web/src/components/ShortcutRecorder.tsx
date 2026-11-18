import { Keyboard, X } from "lucide-react";
import { useState, type KeyboardEvent } from "react";
import { formatShortcutLabel, shortcutFromKeyEvent } from "../lib/shortcuts";
import { IconButton } from "./IconButton";

export function ShortcutRecorder({
  label,
  value,
  recordingLabel,
  clearLabel,
  platform,
  onChange
}: {
  label: string;
  value: string | null;
  recordingLabel: string;
  clearLabel: string;
  platform: string;
  onChange: (value: string | null) => void;
}) {
  const [recording, setRecording] = useState(false);

  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (!recording) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      setRecording(false);
      return;
    }
    const shortcut = shortcutFromKeyEvent(event);
    if (!shortcut) return;
    onChange(shortcut);
    setRecording(false);
  };

  return (
    <div className="flex min-w-0 items-center gap-1.5">
      <button
        type="button"
        aria-label={label}
        className="control flex min-w-0 flex-1 items-center justify-start gap-2 px-2.5 text-left text-[12px]"
        style={recording ? { borderColor: "var(--accent)" } : undefined}
        onClick={(event) => {
          event.currentTarget.focus();
          setRecording(true);
        }}
        onBlur={() => setRecording(false)}
        onKeyDown={onKeyDown}
      >
        <Keyboard size={12} className="shrink-0 text-tertiary" />
        <span className="truncate">
          {recording
            ? recordingLabel
            : formatShortcutLabel(value, platform) || recordingLabel}
        </span>
      </button>
      {value && (
        <IconButton
          label={clearLabel}
          className="!h-9 !w-9"
          onClick={() => {
            setRecording(false);
            onChange(null);
          }}
        >
          <X size={12} />
        </IconButton>
      )}
    </div>
  );
}
