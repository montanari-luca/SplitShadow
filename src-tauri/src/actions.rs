//! IPC action model (mirrors `Command` in src/matcher/types.ts) and its dispatcher.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::state::AppState;
use crate::wm::{Layout, WindowId, WindowInfo};
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
    WsSave { name: String },
    WsOpen { name: String },
    WsUpdate { name: Option<String> },
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
    PaletteContext {
        windows,
        workspaces: Vec::new(),
        active_workspace: None,
        target_id,
        linked: false,
        update: None,
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
        .ok_or_else(|| "No window was selected. Click a window, then open the palette.".to_string())
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
        Action::Split { .. }
        | Action::Unsplit
        | Action::WsSave { .. }
        | Action::WsOpen { .. }
        | Action::WsUpdate { .. }
        | Action::WsDelete { .. }
        | Action::InstallUpdate
        | Action::CheckUpdate => return Err("Not available yet.".into()),
    };

    palette::hide(app);
    if let Some(t) = state.target.lock().unwrap().as_ref() {
        let _ = state.backend.focus(t.id);
    }
    Ok(message)
}
