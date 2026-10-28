import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "../types";

export function useSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void invoke<Settings>("get_settings").then(setSettings);
  }, []);

  /** Save a change. Resolves false (and sets `error`) if Rust rejected it, e.g. a taken hotkey. */
  const update = useCallback(
    async (patch: Partial<Settings>) => {
      if (!settings) return false;
      try {
        const saved = await invoke<Settings>("save_settings", { settings: { ...settings, ...patch } });
        setSettings(saved);
        setError(null);
        return true;
      } catch (e) {
        setError(String(e));
        return false;
      }
    },
    [settings],
  );

  return { settings, update, error, clearError: () => setError(null) };
}
