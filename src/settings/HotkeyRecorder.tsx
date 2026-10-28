import { useEffect, useState } from "react";
import { eventToShortcut, shortcutParts } from "./hotkey";
import { Kbd, isMac } from "./ui";

/** Click, then press the new combination. Esc cancels. */
export function HotkeyRecorder({ value, onChange }: { value: string; onChange: (v: string) => Promise<boolean> }) {
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.code === "Escape") return setRecording(false);
      const sc = eventToShortcut(e);
      if (!sc) return;
      setRecording(false);
      void onChange(sc);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onChange]);

  return (
    <button
      onClick={() => setRecording((r) => !r)}
      className="rounded-lg px-3 py-1.5"
      style={{ background: recording ? "var(--glyph-fill)" : "var(--chip)", color: recording ? "#07130a" : "var(--text)" }}
    >
      {recording ? "Press a shortcut... (Esc to cancel)" : <Kbd parts={shortcutParts(value, isMac)} />}
    </button>
  );
}
