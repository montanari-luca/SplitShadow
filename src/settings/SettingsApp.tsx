import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { About } from "./About";
import { General } from "./General";
import { Permissions } from "./Permissions";
import { Updates } from "./Updates";
import { Welcome } from "./Welcome";
import { Workspaces } from "./Workspaces";
import { isMac } from "./ui";
import { useSettings } from "./useSettings";

type Tab = "general" | "workspaces" | "updates" | "about" | "welcome";

const TABS: Array<{ id: Exclude<Tab, "welcome">; label: string }> = [
  { id: "general", label: "General" },
  { id: "workspaces", label: "Workspaces" },
  { id: "updates", label: "Updates" },
  { id: "about", label: "About" },
];

function initialTab(): Tab {
  const t = new URLSearchParams(location.search).get("tab");
  return (["general", "workspaces", "updates", "about", "welcome"] as const).find((x) => x === t) ?? "general";
}

export function SettingsApp() {
  const s = useSettings();
  const [tab, setTab] = useState<Tab>(initialTab);

  useEffect(() => {
    const un = listen<string>("settings://tab", (e) => setTab(e.payload as Tab));
    return () => void un.then((f) => f());
  }, []);

  if (!s.settings) return null;

  if (tab === "welcome") {
    return (
      <div className="h-full overflow-y-auto p-8">
        <Welcome s={s} />
      </div>
    );
  }

  return (
    <div className="flex h-full">
      <nav className="w-48 shrink-0 border-r p-3 pt-6" style={{ borderColor: "var(--border)" }}>
        {TABS.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className="mb-1 block w-full rounded-lg px-3 py-2 text-left text-[14px]"
            style={{ background: tab === t.id ? "var(--row-active)" : "transparent" }}
          >
            {t.label}
          </button>
        ))}
      </nav>
      <main className="min-w-0 flex-1 overflow-y-auto p-8">
        {tab === "general" && (
          <>
            <General s={s} />
            {isMac && <Permissions />}
          </>
        )}
        {tab === "workspaces" && <Workspaces />}
        {tab === "updates" && <Updates />}
        {tab === "about" && <About />}
      </main>
    </div>
  );
}
