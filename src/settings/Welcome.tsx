import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { HotkeyRecorder } from "./HotkeyRecorder";
import { Permissions } from "./Permissions";
import { Button, Row, Section, isMac } from "./ui";
import type { useSettings } from "./useSettings";

export function Welcome({ s }: { s: ReturnType<typeof useSettings> }) {
  const { settings, update, error, clearError } = s;
  const [finishError, setFinishError] = useState<string | null>(null);
  if (!settings) return null;

  const done = async () => {
    try {
      await invoke("finish_onboarding");
    } catch (e) {
      setFinishError(String(e));
    }
  };

  return (
    <div className="mx-auto max-w-lg pt-2">
      <div className="flex flex-col items-center text-center">
        <img src="/splitshadow-logo.png" alt="" width={80} height={80} className="rounded-[18px] shadow-lg" />
        <h1 className="mt-4 text-2xl font-semibold">Welcome to SplitShadow</h1>
        <p className="mt-3 text-[14px] leading-relaxed" style={{ color: "var(--muted)" }}>
          Press your shortcut anywhere to open the palette, then type what you want: <i>left half</i>, <i>split with Slack</i>, <i>open workspace Dev</i>.
        </p>
      </div>

      <div className="mt-6">
        <Section title="Your shortcut">
          <Row label="Open the palette" hint="Click it to record a different shortcut. Include a modifier key.">
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
      </div>

      {isMac && <Permissions />}

      {finishError && (
        <div className="mb-4 rounded-lg px-3 py-2 text-[13px]" style={{ background: "rgba(255,69,58,0.15)", color: "#ff6b61" }}>
          {finishError}
        </div>
      )}
      <div className="flex justify-center">
        <Button kind="primary" onClick={() => void done()}>
          Get started
        </Button>
      </div>
    </div>
  );
}
