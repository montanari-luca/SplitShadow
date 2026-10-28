//! Window management core. Everything above `WindowBackend` is platform agnostic.

pub mod engine;
pub mod layouts;
pub mod link;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(test)]
pub mod fake;

use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub use layouts::{Layout, Rect};

/// Opaque, backend-defined window identifier (stable while the window exists).
pub type WindowId = u64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: WindowId,
    pub pid: i32,
    /// Display name of the owning app.
    pub app: String,
    /// Stable app identity: bundle id on macOS, exe path on Windows.
    pub app_id: String,
    pub title: String,
    /// Global screen coordinates, top-left origin.
    pub rect: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub id: u32,
    pub frame: Rect,
    /// Frame minus menu bar, dock and taskbar.
    pub work_area: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowEvent {
    /// Position or size changed.
    Geometry(WindowId),
    /// Window closed, minimized, or its app quit.
    Gone(WindowId),
}

pub type EventSink = Arc<dyn Fn(WindowEvent) + Send + Sync>;

pub trait WindowBackend: Send + Sync {
    /// Whether the OS lets us control other apps (macOS Accessibility).
    fn is_trusted(&self) -> bool;
    /// Trigger the OS permission prompt where one exists.
    fn request_trust(&self);

    fn frontmost_window(&self) -> Option<WindowInfo>;
    /// Visible, standard, non-minimized windows, front to back.
    fn list_windows(&self) -> Vec<WindowInfo>;
    fn window(&self, id: WindowId) -> Option<WindowInfo>;
    fn set_rect(&self, id: WindowId, rect: Rect) -> Result<(), String>;
    fn focus(&self, id: WindowId) -> Result<(), String>;
    fn minimize(&self, id: WindowId) -> Result<(), String>;
    fn monitors(&self) -> Vec<Monitor>;
    /// Start an app by its `app_id` (no-op if it is already running).
    fn launch(&self, app_id: &str) -> Result<(), String>;
    /// Subscribe to geometry/close events of one window. Dropping the guard unsubscribes.
    fn watch(&self, id: WindowId, sink: EventSink) -> Result<Box<dyn Send>, String>;
}

pub fn create_backend() -> Arc<dyn WindowBackend> {
    #[cfg(target_os = "macos")]
    return Arc::new(macos::MacBackend::new());
    #[cfg(target_os = "windows")]
    return Arc::new(windows::WinBackend::new());
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    compile_error!("SplitShadow supports macOS and Windows only");
}

/// The monitor whose frame overlaps `rect` the most, falling back to the first.
pub fn monitor_for<'a>(monitors: &'a [Monitor], rect: &Rect) -> Option<&'a Monitor> {
    monitors
        .iter()
        .max_by(|a, b| {
            a.frame
                .overlap_area(rect)
                .partial_cmp(&b.frame.overlap_area(rect))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// The window the palette should act on: the focused one, or when focus is on SplitShadow
/// itself (its own settings window, or a tray click) the frontmost window of any other app.
pub fn pick_target(backend: &dyn WindowBackend) -> Option<WindowInfo> {
    backend.frontmost_window().or_else(|| backend.list_windows().into_iter().next())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fake::FakeBackend;

    #[test]
    fn target_is_the_focused_window() {
        let b = FakeBackend::with_monitor(Rect::new(0.0, 0.0, 1000.0, 800.0));
        b.add_window(1, "Notes", Rect::new(0.0, 0.0, 100.0, 100.0));
        b.add_window(2, "Slack", Rect::new(0.0, 0.0, 100.0, 100.0));
        *b.frontmost.lock().unwrap() = Some(2);
        assert_eq!(pick_target(&*b).unwrap().id, 2);
    }

    #[test]
    fn falls_back_to_the_topmost_other_window() {
        let b = FakeBackend::with_monitor(Rect::new(0.0, 0.0, 1000.0, 800.0));
        b.add_window(1, "Notes", Rect::new(0.0, 0.0, 100.0, 100.0));
        b.add_window(2, "Slack", Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(pick_target(&*b).unwrap().id, 1);
    }

    #[test]
    fn no_windows_means_no_target() {
        let b = FakeBackend::with_monitor(Rect::new(0.0, 0.0, 1000.0, 800.0));
        assert!(pick_target(&*b).is_none());
    }
}
