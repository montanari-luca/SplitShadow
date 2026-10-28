use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

use crate::state::AppState;

pub const PALETTE: &str = "palette";
pub const WIDTH: f64 = 680.0;
pub const HEIGHT: f64 = 540.0;
/// Transparent margin around the panel (the `p-1` in Palette.tsx).
const PANEL_INSET: f64 = 4.0;
/// Height of the panel with a full result list: input 64 + list 368 (8 rows of 44 plus
/// padding, incl. its top border) + footer 37 + 2 outer borders.
/// The panel is centered at this height, so the input bar stays put while the list shrinks.
const PANEL_MAX_HEIGHT: f64 = 471.0;
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
    let target = crate::wm::pick_target(&*state.backend);
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
    let (x, y) = origin(
        (m.position().x as f64, m.position().y as f64),
        (m.size().width as f64, m.size().height as f64),
        scale,
    );
    let _ = w.set_position(PhysicalPosition::new(x, y));
}

/// Top-left of the palette window, in physical pixels, for a monitor at `pos` of `size`.
/// Centers the full-height panel on the monitor; on short screens the window is pushed up
/// just enough to stay fully visible.
fn origin(pos: (f64, f64), size: (f64, f64), scale: f64) -> (i32, i32) {
    let (mx, my) = pos;
    let (mw, mh) = size;
    let x = mx + (mw - WIDTH * scale) / 2.0;
    let ideal = my + mh / 2.0 - (PANEL_INSET + PANEL_MAX_HEIGHT / 2.0) * scale;
    let y = ideal.min(my + mh - HEIGHT * scale).max(my);
    (x.round() as i32, y.round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vertical middle of the visible panel for a window placed at `y`.
    fn panel_middle(y: i32, scale: f64) -> f64 {
        y as f64 + (PANEL_INSET + PANEL_MAX_HEIGHT / 2.0) * scale
    }

    #[test]
    fn panel_is_centered_on_a_laptop_screen() {
        // 1470x956 points at 2x, primary display.
        let (x, y) = origin((0.0, 0.0), (2940.0, 1912.0), 2.0);
        assert_eq!(x, (2940.0 - 680.0 * 2.0) as i32 / 2);
        assert!((panel_middle(y, 2.0) - 956.0).abs() <= 1.0);
    }

    #[test]
    fn panel_is_centered_on_a_secondary_monitor() {
        // Second 1920x1080 display to the right at 1x.
        let (x, y) = origin((2940.0, 0.0), (1920.0, 1080.0), 1.0);
        assert_eq!(x, 2940 + (1920 - 680) / 2);
        assert!((panel_middle(y, 1.0) - 540.0).abs() <= 1.0);
    }

    #[test]
    fn short_screens_keep_the_window_on_screen() {
        let (_, y) = origin((0.0, 0.0), (1366.0, 600.0), 1.0);
        assert!(y >= 0);
        assert!(y as f64 + HEIGHT <= 600.0 || y == 0);
    }
}
