import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Permissions, useAccessibility } from "./Permissions";
import { shortcutParts } from "./hotkey";
import { Button, Kbd, isMac } from "./ui";
import type { Settings } from "../types";

export function Welcome({ settings }: { settings: Settings }) {
  const trusted = useAccessibility();
  const done = async () => {
    await invoke("finish_onboarding");
    await getCurrentWindow().close();
  };
  return (
    <div className="mx-auto max-w-md pt-4">
      <div className="flex flex-col items-center text-center">
        <img src="/splitshadow-logo.png" alt="" width={80} height={80} className="rounded-[18px] shadow-lg" />
        <h1 className="mt-4 text-2xl font-semibold">Welcome to SplitShadow</h1>
        <p className="mt-3 text-[14px] leading-relaxed" style={{ color: "var(--muted)" }}>
          Press your shortcut anywhere to open the palette, then type what you want: <i>left half</i>, <i>split with Slack</i>, <i>open workspace Dev</i>.
        </p>
        <div className="mt-5 flex items-center gap-3 text-[14px]">
          Your shortcut <Kbd parts={shortcutParts(settings.hotkey, isMac)} />
        </div>
      </div>
      {isMac && (
        <div className="mt-8">
          <Permissions />
        </div>
      )}
      <div className="mt-6 flex justify-center">
        <Button kind="primary" onClick={() => void done()} disabled={isMac && trusted === false}>
          {isMac && trusted === false ? "Grant access to continue" : "Get started"}
        </Button>
      </div>
    </div>
  );
}
