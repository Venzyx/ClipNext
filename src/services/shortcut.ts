const MODIFIER_CODES = /^(Control|Shift|Alt|Meta)(Left|Right)$/;

const KEY_LABELS: Record<string, string> = {
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Semicolon: ";",
  Quote: "'",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
};

/**
 * Turns a key press into the accelerator format the backend expects,
 * e.g. "Control+Shift+KeyV". Returns null until a non-modifier key is
 * pressed together with at least one modifier.
 */
export function comboFromEvent(e: KeyboardEvent): string | null {
  if (MODIFIER_CODES.test(e.code)) return null;
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Control");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");
  if (mods.length === 0) return null;
  return [...mods, e.code].join("+");
}

/** "Control+Shift+KeyV" -> "Ctrl+Shift+V" */
export function prettyShortcut(combo: string): string {
  return combo
    .split("+")
    .map((part) => {
      if (part === "Control") return "Ctrl";
      if (part === "Super") return "Win";
      if (part.startsWith("Key")) return part.slice(3);
      if (part.startsWith("Digit")) return part.slice(5);
      return KEY_LABELS[part] ?? part;
    })
    .join("+");
}
