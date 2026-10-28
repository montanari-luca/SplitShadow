import { describe, expect, it } from "vitest";
import { eventToShortcut, shortcutParts } from "./hotkey";

const ev = (code: string, mods: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>> = {}) => ({
  code,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe("eventToShortcut", () => {
  it("builds combinations in a stable modifier order", () => {
    expect(eventToShortcut(ev("Space", { ctrlKey: true }))).toBe("Ctrl+Space");
    expect(eventToShortcut(ev("KeyK", { metaKey: true, shiftKey: true }))).toBe("Shift+Super+K");
    expect(eventToShortcut(ev("Digit1", { ctrlKey: true, altKey: true }))).toBe("Ctrl+Alt+1");
    expect(eventToShortcut(ev("ArrowUp", { altKey: true }))).toBe("Alt+Up");
  });

  it("ignores modifier-only presses", () => {
    expect(eventToShortcut(ev("ControlLeft", { ctrlKey: true }))).toBeNull();
    expect(eventToShortcut(ev("MetaRight", { metaKey: true }))).toBeNull();
  });

  it("rejects bare typing keys but allows function keys", () => {
    expect(eventToShortcut(ev("KeyA"))).toBeNull();
    expect(eventToShortcut(ev("Space"))).toBeNull();
    expect(eventToShortcut(ev("F5"))).toBe("F5");
  });

  it("rejects unknown keys", () => {
    expect(eventToShortcut(ev("MediaPlayPause", { ctrlKey: true }))).toBeNull();
  });
});

describe("shortcutParts", () => {
  it("uses symbols on macOS", () => {
    expect(shortcutParts("Ctrl+Alt+Space", true)).toEqual(["⌃", "⌥", "Space"]);
  });
  it("uses names on Windows", () => {
    expect(shortcutParts("Super+Shift+K", false)).toEqual(["Win", "Shift", "K"]);
  });
});
