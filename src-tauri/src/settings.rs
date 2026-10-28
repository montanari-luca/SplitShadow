use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
pub const DEFAULT_HOTKEY: &str = "Ctrl+Space";
#[cfg(not(target_os = "macos"))]
pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+Space";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub hotkey: String,
    /// Fraction of the work area used by "almost maximize" (0.5 to 1.0).
    pub almost_max: f64,
    pub launch_at_login: bool,
    /// "system", "light" or "dark".
    pub theme: String,
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: DEFAULT_HOTKEY.into(),
            almost_max: 0.9,
            launch_at_login: false,
            theme: "system".into(),
            onboarded: false,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Clamp values that the UI could send out of range.
    pub fn sanitized(mut self) -> Self {
        self.almost_max = self.almost_max.clamp(0.5, 1.0);
        if !matches!(self.theme.as_str(), "system" | "light" | "dark") {
            self.theme = "system".into();
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let s = Settings::load(Path::new("/definitely/not/here.json"));
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn partial_json_fills_defaults() {
        let s: Settings = serde_json::from_str(r#"{"hotkey":"Alt+Space"}"#).unwrap();
        assert_eq!(s.hotkey, "Alt+Space");
        assert_eq!(s.almost_max, 0.9);
    }

    #[test]
    fn sanitize_clamps() {
        let s = Settings { almost_max: 3.0, theme: "neon".into(), ..Default::default() }.sanitized();
        assert_eq!(s.almost_max, 1.0);
        assert_eq!(s.theme, "system");
    }

    #[test]
    fn round_trip_on_disk() {
        let dir = std::env::temp_dir().join(format!("splitshadow-test-{}", std::process::id()));
        let path = dir.join("settings.json");
        let s = Settings { hotkey: "Alt+K".into(), ..Default::default() };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        let _ = fs::remove_dir_all(dir);
    }
}
