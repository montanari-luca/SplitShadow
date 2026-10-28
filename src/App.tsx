import { getCurrentWindow } from "@tauri-apps/api/window";
import { Palette } from "./palette/Palette";
import { SettingsApp } from "./settings/SettingsApp";

export default function App() {
  return getCurrentWindow().label === "palette" ? <Palette /> : <SettingsApp />;
}
