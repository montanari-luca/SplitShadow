import { HotkeyRecorder } from "./HotkeyRecorder";
import { Button, Row, Section, Toggle } from "./ui";
import type { useSettings } from "./useSettings";

type S = ReturnType<typeof useSettings>;

export function General({ s }: { s: S }) {
  const { settings, update, error, clearError } = s;
  if (!settings) return null;
  return (
    <>
      <Section title="Shortcut">
        <Row label="Open the palette" hint="Works from any app while SplitShadow is running. Include at least one modifier key.">
          <HotkeyRecorder value={settings.hotkey} onChange={(hotkey) => update({ hotkey })} />
        </Row>
        {error && (
          <div className="flex items-center justify-between gap-4 px-4 py-3 text-[13px]" style={{ background: "rgba(255,69,58,0.15)", color: "#ff6b61" }}>
            <span>{error}</span>
            <button onClick={clearError} className="underline">
              Dismiss
            </button>
          </div>
        )}
      </Section>
      <Section title="Behavior">
        <Row label="Launch at login" hint="Start SplitShadow in the background when you sign in.">
          <Toggle on={settings.launchAtLogin} onChange={(launchAtLogin) => void update({ launchAtLogin })} />
        </Row>
        <Row label="Almost maximize size" hint="How much of the screen the almost maximize action fills.">
          <div className="flex items-center gap-3">
            <input
              type="range"
              min={60}
              max={100}
              value={Math.round(settings.almostMax * 100)}
              onChange={(e) => void update({ almostMax: Number(e.target.value) / 100 })}
              className="w-36 accent-[var(--glyph-fill)]"
            />
            <span className="w-10 text-right tabular-nums">{Math.round(settings.almostMax * 100)}%</span>
          </div>
        </Row>
        <Row label="Appearance">
          <div className="flex gap-1.5">
            {(["system", "light", "dark"] as const).map((t) => (
              <Button key={t} kind={settings.theme === t ? "primary" : "default"} onClick={() => void update({ theme: t })}>
                {t[0].toUpperCase() + t.slice(1)}
              </Button>
            ))}
          </div>
        </Row>
      </Section>
    </>
  );
}
