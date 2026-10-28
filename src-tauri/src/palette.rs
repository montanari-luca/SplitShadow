use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

use crate::state::AppState;

pub const PALETTE: &str = "palette";
pub const WIDTH: f64 = 680.0;
pub const HEIGHT: f64 = 540.0;
/// Distance from the window top to the vertical middle of the input bar.
const INPUT_CENTER_OFFSET: f64 = 36.0;
/// Ignore focus-loss events this long after showing, the OS can emit one while activating.
const BLUR_GRACE: Duration = Duration::from_millis(250);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ShowPayload {
    pub trusted: bool,
    pub target_app: Option<String>,
    pub target_title: Option<String>,
}

pub fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(PALETTE)
}

pub fn is_visible(app: &AppHandle) -> bool {
    window(app).and_then(|w| w.is_visible().ok()).unwrap_or(false)
}

pub fn toggle(app: &AppHandle) {
    if is_visible(app) { hide(app) } else { show(app) }
}

pub fn hide(app: &AppHandle) {
    if let Some(w) = window(app) {
        let _ = w.hide();
    }
}

pub fn show(app: &AppHandle) {
    let Some(w) = window(app) else { return };
    let state = app.state::<AppState>();
    let target = state.backend.frontmost_window();
    let payload = ShowPayload {
        trusted: state.backend.is_trusted(),
        target_app: target.as_ref().map(|t| t.app.clone()),
        target_title: target.as_ref().map(|t| t.title.clone()),
    };
    *state.target.lock().unwrap() = target;
    *state.shown_at.lock().unwrap() = Some(Instant::now());

    center_on_cursor_monitor(app, &w);
    let _ = w.show();
    let _ = w.set_focus();
    let _ = app.emit_to(PALETTE, "palette://show", payload);
}

/// Called on window focus loss: close unless we only just opened.
pub fn on_blur(app: &AppHandle) {
    let state = app.state::<AppState>();
    let recent = state
        .shown_at
        .lock()
        .unwrap()
        .map(|t| t.elapsed() < BLUR_GRACE)
        .unwrap_or(false);
    if !recent {
        hide(app);
    }
}

fn center_on_cursor_monitor(app: &AppHandle, w: &WebviewWindow) {
    let _ = w.set_size(LogicalSize::new(WIDTH, HEIGHT));
    let cursor = app.cursor_position().ok();
    let monitor = cursor
        .and_then(|c| app.monitor_from_point(c.x, c.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(m) = monitor else {
        let _ = w.center();
        return;
    };
    let scale = m.scale_factor();
    let (mx, my) = (m.position().x as f64, m.position().y as f64);
    let (mw, mh) = (m.size().width as f64, m.size().height as f64);
    let (ww, wh) = (WIDTH * scale, HEIGHT * scale);
    let x = mx + (mw - ww) / 2.0;
    // Input bar centered vertically, then clamped so the result list stays on screen.
    let ideal = my + mh / 2.0 - INPUT_CENTER_OFFSET * scale;
    let y = ideal.min(my + mh - wh).max(my);
    let _ = w.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
}
