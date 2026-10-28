//! IPC action model (mirrors `Command` in src/matcher/types.ts) and its dispatcher.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::state::AppState;
use crate::wm::link::Axis;
use crate::wm::{Layout, WindowId, WindowInfo};
use crate::workspaces::{self, OpenOptions};
use crate::{palette, windows};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", rename_all_fields = "camelCase")]
pub enum Action {
    Layout { layout: Layout },
    Minimize,
    Resize { step: f64 },
    Display { delta: i32 },
    Restore,
    Split { window_id: WindowId },
    Unsplit,
    WsSave { name: String, window_ids: Option<Vec<WindowId>> },
    WsOpen { name: String },
    WsUpdate { name: Option<String>, window_ids: Option<Vec<WindowId>> },
    WsDelete { name: String },
    Settings,
    Quit,
    InstallUpdate,
    CheckUpdate,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowRef {
    pub id: WindowId,
    pub app: String,
    pub title: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaletteContext {
    pub windows: Vec<WindowRef>,
    pub workspaces: Vec<String>,
    pub active_workspace: Option<String>,
    pub target_id: Option<WindowId>,
    pub linked: bool,
    pub update: Option<String>,
}

pub fn context(app: &AppHandle) -> PaletteContext {
    let state = app.state::<AppState>();
    let windows = if state.backend.is_trusted() {
        state
            .backend
            .list_windows()
            .into_iter()
            .map(|w| WindowRef { id: w.id, app: w.app, title: w.title })
            .collect()
    } else {
        Vec::new()
    };
    let target_id = state.target.lock().unwrap().as_ref().map(|t| t.id);
    let store = state.workspaces.lock().unwrap();
    PaletteContext {
        windows,
        workspaces: store.names(),
        active_workspace: store.active.clone(),
        target_id,
        linked: target_id.map(|id| state.link.is_linked(id)).unwrap_or(false),
        update: crate::updater::pending_version(app),
    }
}

fn target(state: &AppState) -> Result<WindowInfo, String> {
    if !state.backend.is_trusted() {
        return Err("SplitShadow needs Accessibility access to move windows.".into());
    }
    state
        .target
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "There is no window to act on. Open a window in another app, then try again.".to_string())
}

/// Run one action. On success the palette is closed and focus returns to the target.
pub fn run(app: &AppHandle, action: Action) -> Result<String, String> {
    let state = app.state::<AppState>();
    let almost = state.settings.lock().unwrap().almost_max;

    let message = match action {
        Action::Layout { layout } => {
            let t = target(&state)?;
            state.engine.layout(t.id, layout, almost)?;
            format!("{} moved", t.app)
        }
        Action::Minimize => {
            let t = target(&state)?;
            state.engine.minimize(t.id)?;
            format!("{} hidden", t.app)
        }
        Action::Resize { step } => {
            let t = target(&state)?;
            state.engine.resize(t.id, step)?;
            format!("{} resized", t.app)
        }
        Action::Display { delta } => {
            let t = target(&state)?;
            state.engine.move_display(t.id, delta)?;
            format!("{} moved to another display", t.app)
        }
        Action::Restore => {
            let t = target(&state)?;
            state.engine.restore(t.id)?;
            format!("{} restored", t.app)
        }
        Action::Settings => {
            palette::hide(app);
            windows::open_settings(app, None);
            return Ok(String::new());
        }
        Action::Quit => {
            app.exit(0);
            return Ok(String::new());
        }
        Action::Split { window_id } => {
            let t = target(&state)?;
            let partner = state.backend.window(window_id).ok_or("That window is gone.")?;
            state.link.split(t.id, window_id, Axis::Side, almost)?;
            let _ = state.backend.focus(window_id);
            format!("{} and {} are linked", t.app, partner.app)
        }
        Action::Unsplit => {
            state.link.unlink();
            "Split unlinked".to_string()
        }
        Action::WsSave { name, window_ids } => {
            let name = workspaces::clean_name(&name)?;
            save_workspace(&state, &name, window_ids.as_deref())?
        }
        Action::WsUpdate { name, window_ids } => {
            let name = match name {
                Some(n) => n,
                None => state
                    .workspaces
                    .lock()
                    .unwrap()
                    .active
                    .clone()
                    .ok_or("No active workspace. Open or save one first, or name it: update workspace <name>")?,
            };
            if state.workspaces.lock().unwrap().find(&name).is_none() {
                return Err(format!("No workspace named \"{name}\"."));
            }
            save_workspace(&state, &name, window_ids.as_deref())?
        }
        Action::WsOpen { name } => {
            let ws = state
                .workspaces
                .lock()
                .unwrap()
                .find(&name)
                .cloned()
                .ok_or_else(|| format!("No workspace named \"{name}\"."))?;
            if !state.backend.is_trusted() {
                return Err("SplitShadow needs Accessibility access to move windows.".into());
            }
            // Apps may take a while to launch, so get out of the way first.
            palette::hide(app);
            let report = workspaces::open(&*state.backend, &ws, &OpenOptions::default());
            state.link.unlink();
            if let Some((i, j)) = ws.link
                && let (Some(Some(a)), Some(Some(b))) = (report.ids.get(i), report.ids.get(j))
            {
                let _ = state.link.link_existing(*a, *b);
            }
            {
                let mut store = state.workspaces.lock().unwrap();
                store.active = Some(ws.name.clone());
                store.save(&state.workspaces_path)?;
            }
            return Ok(if report.missing.is_empty() {
                format!("Opened {}", ws.name)
            } else {
                format!("Opened {} (could not find: {})", ws.name, report.missing.join(", "))
            });
        }
        Action::WsDelete { name } => {
            let mut store = state.workspaces.lock().unwrap();
            let removed = store.delete(&name)?;
            store.save(&state.workspaces_path)?;
            return {
                drop(store);
                palette::hide(app);
                Ok(format!("Deleted {removed}"))
            };
        }
        Action::CheckUpdate => {
            let st = tauri::async_runtime::block_on(crate::updater::check(app));
            return match (st.error, st.version) {
                (Some(e), _) => Err(e),
                (None, Some(v)) => Ok(format!("Version {v} is available")),
                (None, None) => Ok(format!("SplitShadow {} is up to date", st.current)),
            };
        }
        Action::InstallUpdate => {
            palette::hide(app);
            return tauri::async_runtime::block_on(crate::updater::install(app)).map(|_| String::new());
        }
    };

    palette::hide(app);
    if let Some(t) = state.target.lock().unwrap().as_ref() {
        let _ = state.backend.focus(t.id);
    }
    Ok(message)
}

fn save_workspace(state: &AppState, name: &str, ids: Option<&[WindowId]>) -> Result<String, String> {
    let t = target(state)?;
    let (windows, link) = workspaces::capture(&*state.backend, t.id, ids, state.link.pair())?;
    let count = windows.len();
    let mut store = state.workspaces.lock().unwrap();
    let existed = store.find(name).is_some();
    let ws = store.upsert(name, windows, link)?.name.clone();
    store.save(&state.workspaces_path)?;
    Ok(format!("{} \"{ws}\" ({count} window{})", if existed { "Updated" } else { "Saved" }, if count == 1 { "" } else { "s" }))
}
