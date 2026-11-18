export type ShortcutKeyEvent = {
  code: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
};

const modifierCodes = new Set([
  "AltLeft",
  "AltRight",
  "ControlLeft",
  "ControlRight",
  "MetaLeft",
  "MetaRight",
  "ShiftLeft",
  "ShiftRight"
]);

export function shortcutFromKeyEvent(event: ShortcutKeyEvent): string | null {
  if (modifierCodes.has(event.code)) return null;

  const key = normalizeCode(event.code);
  if (!key) return null;

  const modifiers = [
    event.ctrlKey ? "Control" : null,
    event.altKey ? "Alt" : null,
    event.shiftKey ? "Shift" : null,
    event.metaKey ? "Super" : null
  ].filter(Boolean);

  const hasModifier = modifiers.length > 0;
  const standaloneFunctionKey = /^F(?:1|2|3|4|5|6|7|8|9|10|11|12)$/.test(key);
  if (!hasModifier && !standaloneFunctionKey) return null;

  return [...modifiers, key].join("+");
}

export function formatShortcutLabel(value: string | null | undefined, platform: string): string {
  if (!value) return "";
  return value
    .split("+")
    .map((token) => {
      if (platform === "macos") {
        if (token === "Super") return "⌘";
        if (token === "Control") return "⌃";
        if (token === "Alt") return "⌥";
        if (token === "Shift") return "⇧";
      }
      if (token.startsWith("Key")) return token.slice(3);
      if (token.startsWith("Digit")) return token.slice(5);
      return token;
    })
    .join(platform === "macos" ? " " : " + ");
}

function normalizeCode(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F(?:1|2|3|4|5|6|7|8|9|10|11|12)$/.test(code)) return code;
  if (
    [
      "Backquote",
      "Backslash",
      "BracketLeft",
      "BracketRight",
      "Comma",
      "Enter",
      "Equal",
      "Minus",
      "Period",
      "Quote",
      "Semicolon",
      "Slash",
      "Space",
      "Tab"
    ].includes(code)
  ) {
    return code;
  }
  return null;
}
