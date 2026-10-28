//! Applies layout actions to a window through a `WindowBackend`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::layouts::{self, Layout};
use super::{Monitor, Rect, WindowBackend, WindowId, monitor_for};

pub struct Engine {
    backend: Arc<dyn WindowBackend>,
    /// Geometry before the last action, per window, for "restore".
    history: Mutex<HashMap<WindowId, Rect>>,
}

impl Engine {
    pub fn new(backend: Arc<dyn WindowBackend>) -> Self {
        Self { backend, history: Mutex::new(HashMap::new()) }
    }

    fn context(&self, id: WindowId) -> Result<(Rect, Vec<Monitor>, Monitor), String> {
        let win = self.backend.window(id).ok_or("That window is gone")?;
        let monitors = self.backend.monitors();
        let mon = *monitor_for(&monitors, &win.rect).ok_or("No display found")?;
        Ok((win.rect, monitors, mon))
    }

    fn move_to(&self, id: WindowId, from: Rect, to: Rect) -> Result<Rect, String> {
        self.history.lock().unwrap().insert(id, from);
        self.backend.set_rect(id, to)?;
        // Apps may clamp the size, so report what actually happened.
        Ok(self.backend.window(id).map(|w| w.rect).unwrap_or(to))
    }

    pub fn layout(&self, id: WindowId, layout: Layout, almost_max: f64) -> Result<Rect, String> {
        let (cur, _, mon) = self.context(id)?;
        let target = layouts::compute(layout, &mon.work_area, &cur, almost_max);
        self.move_to(id, cur, target)
    }

    /// `step` is a fraction, e.g. 0.1 grows by 10%, -0.1 shrinks by 10%.
    pub fn resize(&self, id: WindowId, step: f64) -> Result<Rect, String> {
        let (cur, _, mon) = self.context(id)?;
        let target = layouts::resize_about_center(&cur, &mon.work_area, step);
        self.move_to(id, cur, target)
    }

    /// Move to the next (`delta` = 1) or previous (-1) display, wrapping around.
    pub fn move_display(&self, id: WindowId, delta: i32) -> Result<Rect, String> {
        let (cur, mut monitors, mon) = self.context(id)?;
        if monitors.len() < 2 {
            return Err("Only one display connected".into());
        }
        // Left to right, so "next" is the display on the right.
        monitors.sort_by(|a, b| a.frame.x.partial_cmp(&b.frame.x).unwrap());
        let idx = monitors.iter().position(|m| m.id == mon.id).unwrap_or(0) as i32;
        let n = monitors.len() as i32;
        let dest = monitors[((idx + delta).rem_euclid(n)) as usize];
        let target = layouts::translate_between(&cur, &mon.work_area, &dest.work_area);
        self.move_to(id, cur, target)
    }

    pub fn restore(&self, id: WindowId) -> Result<Rect, String> {
        let prev = self.history.lock().unwrap().remove(&id).ok_or("Nothing to restore")?;
        self.backend.set_rect(id, prev)?;
        Ok(prev)
    }

    pub fn minimize(&self, id: WindowId) -> Result<(), String> {
        self.backend.minimize(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wm::fake::FakeBackend;
    use crate::wm::Monitor;

    fn setup() -> (Arc<FakeBackend>, Engine) {
        let b = FakeBackend::with_monitor(Rect::new(0.0, 25.0, 1200.0, 775.0));
        b.add_window(1, "Notes", Rect::new(100.0, 100.0, 400.0, 300.0));
        let e = Engine::new(b.clone());
        (b, e)
    }

    #[test]
    fn snaps_left_half() {
        let (b, e) = setup();
        e.layout(1, Layout::LeftHalf, 0.9).unwrap();
        assert_eq!(b.rect_of(1), Rect::new(0.0, 25.0, 600.0, 775.0));
    }

    #[test]
    fn restore_returns_to_previous_geometry() {
        let (b, e) = setup();
        let before = b.rect_of(1);
        e.layout(1, Layout::Maximize, 0.9).unwrap();
        e.restore(1).unwrap();
        assert_eq!(b.rect_of(1), before);
        assert!(e.restore(1).is_err());
    }

    #[test]
    fn reports_clamped_size_from_app_minimum() {
        let (b, e) = setup();
        *b.min_size.lock().unwrap() = (700.0, 0.0);
        let r = e.layout(1, Layout::LeftHalf, 0.9).unwrap();
        assert_eq!(r.w, 700.0);
    }

    #[test]
    fn resize_grows_ten_percent() {
        let (b, e) = setup();
        e.resize(1, 0.1).unwrap();
        assert!((b.rect_of(1).w - 440.0).abs() < 1e-9);
    }

    #[test]
    fn next_display_needs_two_monitors() {
        let (_, e) = setup();
        assert!(e.move_display(1, 1).is_err());
    }

    #[test]
    fn next_display_wraps_and_keeps_relative_position() {
        let (b, e) = setup();
        let second = Rect::new(1200.0, 25.0, 1200.0, 775.0);
        b.monitors.lock().unwrap().push(Monitor { id: 1, frame: second, work_area: second });
        e.move_display(1, 1).unwrap();
        assert_eq!(b.rect_of(1).x, 1300.0);
        e.move_display(1, 1).unwrap();
        assert_eq!(b.rect_of(1).x, 100.0);
    }

    #[test]
    fn missing_window_is_an_error() {
        let (_, e) = setup();
        assert!(e.layout(99, Layout::Center, 0.9).is_err());
    }
}
