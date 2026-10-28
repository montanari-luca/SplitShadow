use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub fn parse(s: &str) -> Result<Shortcut, String> {
    s.parse::<Shortcut>().map_err(|e| format!("invalid shortcut \"{s}\": {e}"))
}

pub fn register(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let sc = parse(hotkey)?;
    app.global_shortcut()
        .register(sc)
        .map_err(|e| format!("could not register {hotkey}: {e}"))
}

/// Swap the global hotkey, restoring the old one if the new one is taken.
pub fn rebind(app: &AppHandle, old: &str, new: &str) -> Result<(), String> {
    let new_sc = parse(new)?;
    if let Ok(old_sc) = parse(old) {
        let _ = app.global_shortcut().unregister(old_sc);
    }
    match app.global_shortcut().register(new_sc) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = register(app, old);
            Err(format!("{new} is unavailable ({e}). Keeping {old}."))
        }
    }
}
