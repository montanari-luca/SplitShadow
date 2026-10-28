import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export function About() {
  const [version, setVersion] = useState("");
  useEffect(() => void invoke<string>("app_version").then(setVersion), []);
  return (
    <div className="flex flex-col items-center pt-8 text-center">
      <img src="/splitshadow-logo.png" alt="SplitShadow" width={96} height={96} className="rounded-[22px] shadow-lg" />
      <h1 className="mt-5 text-2xl font-semibold">SplitShadow</h1>
      <div className="mt-1 text-[13px]" style={{ color: "var(--muted)" }}>
        Version {version}
      </div>
      <p className="mt-5 max-w-sm text-[14px] leading-relaxed" style={{ color: "var(--muted)" }}>
        A fast, keyboard-driven window manager for macOS and Windows.
      </p>
      <div className="mt-8 h-px w-40" style={{ background: "var(--border)" }} />
      <img src="/mintflow-logo.webp" alt="MintFlow Technologies" className="mt-8 h-10" />
      <div className="mt-4 text-[14px]">Developed by MintFlow Technologies</div>
      <div className="mt-1 text-[14px]">Lead developer: Luca Montanari</div>
    </div>
  );
}
