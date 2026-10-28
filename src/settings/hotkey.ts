/** Convert keyboard events to Tauri global-shortcut strings and back to readable labels. */

export interface KeyLike {
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

const MODIFIER_CODES = /^(Control|Alt|Shift|Meta|OS)(Left|Right)?$/;

const NAMED: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Escape: "Escape",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  Minus: "-",
  Equal: "=",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Semicolon: ";",
  Quote: "'",
  Backquote: "`",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
};

function keyName(code: string): string | null {
  if (MODIFIER_CODES.test(code)) return null;
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  return NAMED[code] ?? null;
}

/**
 * Shortcut string for a key press, or null while only modifiers are held or the
 * combination is not usable (a plain letter would steal normal typing everywhere).
 */
export function eventToShortcut(e: KeyLike): string | null {
  const key = keyName(e.code);
  if (!key) return null;
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");
  const isFunctionKey = /^F\d+$/.test(key);
  if (!mods.length && !isFunctionKey) return null;
  return [...mods, key].join("+");
}

const MAC_SYMBOLS: Record<string, string> = { Ctrl: "⌃", Alt: "⌥", Shift: "⇧", Super: "⌘", CommandOrControl: "⌘", Cmd: "⌘", Command: "⌘", Control: "⌃", Option: "⌥" };
const WIN_NAMES: Record<string, string> = { Super: "Win", CommandOrControl: "Ctrl", Cmd: "Win", Command: "Win", Control: "Ctrl", Option: "Alt" };

/** Readable parts of a shortcut, e.g. ["⌃", "Space"] on macOS. */
export function shortcutParts(shortcut: string, mac: boolean): string[] {
  return shortcut
    .split("+")
    .filter(Boolean)
    .map((p) => (mac ? MAC_SYMBOLS[p] ?? p : WIN_NAMES[p] ?? p));
}
