import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button, Section } from "./ui";

interface Summary {
  name: string;
  apps: string[];
  updated: number;
  active: boolean;
}

export function Workspaces() {
  const [items, setItems] = useState<Summary[]>([]);
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [confirm, setConfirm] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => void invoke<Summary[]>("list_workspaces").then(setItems), []);
  useEffect(load, [load]);

  const rename = async (old: string) => {
    try {
      await invoke("rename_workspace", { old, new: draft });
      setEditing(null);
      setError(null);
      load();
    } catch (e) {
      setError(String(e));
    }
  };

  const remove = async (name: string) => {
    await invoke("delete_workspace", { name });
    setConfirm(null);
    load();
  };

  return (
    <Section title="Saved workspaces">
      {items.length === 0 && (
        <div className="px-4 py-8 text-center text-[13.5px]" style={{ color: "var(--muted)" }}>
          No workspaces yet. Open the palette and type <b>save workspace as Name</b>.
        </div>
      )}
      {items.map((w) => (
        <div key={w.name} className="flex items-center justify-between gap-4 border-b px-4 py-3 last:border-b-0" style={{ borderColor: "var(--border)" }}>
          <div className="min-w-0">
            {editing === w.name ? (
              <input
                autoFocus
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                onKeyDown={(e) => (e.key === "Enter" ? void rename(w.name) : e.key === "Escape" && setEditing(null))}
                className="rounded-md px-2 py-1 text-[14px] outline-none"
                style={{ background: "var(--chip)", color: "var(--text)" }}
              />
            ) : (
              <div className="text-[14px]">
                {w.name}
                {w.active && (
                  <span className="ml-2 rounded px-1.5 py-0.5 text-[11px]" style={{ background: "var(--glyph-fill)", color: "#07130a" }}>
                    active
                  </span>
                )}
              </div>
            )}
            <div className="mt-0.5 truncate text-[12.5px]" style={{ color: "var(--muted)" }}>
              {w.apps.join(", ")}
            </div>
          </div>
          <div className="flex shrink-0 gap-2">
            {confirm === w.name ? (
              <>
                <Button kind="danger" onClick={() => void remove(w.name)}>
                  Delete
                </Button>
                <Button onClick={() => setConfirm(null)}>Cancel</Button>
              </>
            ) : editing === w.name ? (
              <>
                <Button kind="primary" onClick={() => void rename(w.name)}>
                  Save
                </Button>
                <Button onClick={() => setEditing(null)}>Cancel</Button>
              </>
            ) : (
              <>
                <Button
                  onClick={() => {
                    setEditing(w.name);
                    setDraft(w.name);
                    setError(null);
                  }}
                >
                  Rename
                </Button>
                <Button onClick={() => setConfirm(w.name)}>Delete</Button>
              </>
            )}
          </div>
        </div>
      ))}
      {error && (
        <div className="px-4 py-3 text-[13px]" style={{ background: "rgba(255,69,58,0.15)", color: "#ff6b61" }}>
          {error}
        </div>
      )}
    </Section>
  );
}
