import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Settings } from "./types";

export function setTheme(theme: string) {
  const root = document.documentElement;
  if (theme === "light" || theme === "dark") root.dataset.theme = theme;
  else delete root.dataset.theme;
}

/** Read the saved theme once, then follow changes pushed from settings. */
export async function applyTheme() {
  try {
    const s = await invoke<Settings>("get_settings");
    setTheme(s.theme);
  } catch {
    /* running outside Tauri */
  }
  void listen<string>("settings://theme", (e) => setTheme(e.payload));
}
