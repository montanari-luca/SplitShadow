use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;

use crate::settings::Settings;
use crate::state::AppState;
use crate::actions::{self, Action, PaletteContext};
use crate::updater::{self, UpdateStatus};
use serde::Serialize;
use tauri::Emitter;
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
    if settings.theme != old.theme {
        let _ = app.emit("settings://theme", settings.theme.clone());
    }
    *state.settings.lock().unwrap() = settings.clone();
    Ok(settings)
}

// Both can block on the OS (Accessibility calls, launching apps), so keep them off the
// main thread, which also services the window observers.
#[tauri::command]
pub async fn palette_context(app: AppHandle) -> Result<PaletteContext, String> {
    tauri::async_runtime::spawn_blocking(move || actions::context(&app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_action(app: AppHandle, action: Action) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || actions::run(&app, action))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn update_status(app: AppHandle) -> UpdateStatus {
    updater::status(&app)
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> UpdateStatus {
    updater::check(&app).await
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    updater::install(&app).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub name: String,
    pub apps: Vec<String>,
    pub updated: u64,
    pub active: bool,
}

#[tauri::command]
pub fn list_workspaces(state: State<AppState>) -> Vec<WorkspaceSummary> {
    let store = state.workspaces.lock().unwrap();
    store
        .workspaces
        .iter()
        .map(|w| WorkspaceSummary {
            name: w.name.clone(),
            apps: w.windows.iter().map(|x| x.app.clone()).collect(),
            updated: w.updated,
            active: store.active.as_deref() == Some(w.name.as_str()),
        })
        .collect()
}

#[tauri::command]
pub fn rename_workspace(state: State<AppState>, old: String, new: String) -> Result<(), String> {
    let mut store = state.workspaces.lock().unwrap();
    store.rename(&old, &new)?;
    store.save(&state.workspaces_path)
}

#[tauri::command]
pub fn delete_workspace(state: State<AppState>, name: String) -> Result<(), String> {
    let mut store = state.workspaces.lock().unwrap();
    store.delete(&name)?;
    store.save(&state.workspaces_path)
}

#[tauri::command]
pub fn finish_onboarding(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    {
        let mut s = state.settings.lock().unwrap();
        s.onboarded = true;
        s.save(&state.settings_path)?;
    }
    // Closed here rather than from the page: the webview has no permission to close windows.
    if let Some(w) = app.get_webview_window(windows::SETTINGS) {
        w.close().map_err(|e| e.to_string())?;
    }
    Ok(())
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
        list_workspaces,
        rename_workspace,
        delete_workspace,
        finish_onboarding,
        update_status,
        check_for_updates,
        install_update,
        palette_context,
        run_action,
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
