use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;

use crate::settings::Settings;
use crate::state::AppState;
use crate::{hotkey, palette, windows};

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> Result<Settings, String> {
    let settings = settings.sanitized();
    let old = state.settings.lock().unwrap().clone();
    if settings.hotkey != old.hotkey {
        hotkey::rebind(&app, &old.hotkey, &settings.hotkey)?;
    }
    if settings.launch_at_login != old.launch_at_login {
        let auto = app.autolaunch();
        let r = if settings.launch_at_login { auto.enable() } else { auto.disable() };
        r.map_err(|e| e.to_string())?;
    }
    settings.save(&state.settings_path)?;
    *state.settings.lock().unwrap() = settings.clone();
    Ok(settings)
}

#[tauri::command]
pub fn palette_hide(app: AppHandle) {
    palette::hide(&app);
}

#[tauri::command]
pub fn open_settings(app: AppHandle, tab: Option<String>) {
    palette::hide(&app);
    windows::open_settings(&app, tab.as_deref());
}

#[tauri::command]
pub fn accessibility_status(state: State<AppState>) -> bool {
    state.backend.is_trusted()
}

#[tauri::command]
pub fn request_accessibility(state: State<AppState>) {
    state.backend.request_trust();
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        get_settings,
        save_settings,
        palette_hide,
        open_settings,
        accessibility_status,
        request_accessibility,
        quit_app,
        app_version,
    ]
}

#[allow(dead_code)]
fn _assert_manager(app: &AppHandle) {
    let _ = app.state::<AppState>();
}
