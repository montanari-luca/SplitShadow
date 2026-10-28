//! In-memory WindowBackend for tests. `set_rect` emits geometry events like the real
//! OS observers do, so loop-guard logic gets exercised.

use std::sync::{Arc, Mutex};

use super::{EventSink, Monitor, Rect, WindowBackend, WindowEvent, WindowId, WindowInfo};

type Sinks = Arc<Mutex<Vec<(u64, WindowId, EventSink)>>>;

#[derive(Default)]
pub struct FakeBackend {
    pub windows: Mutex<Vec<WindowInfo>>,
    pub monitors: Mutex<Vec<Monitor>>,
    pub launched: Mutex<Vec<String>>,
    pub frontmost: Mutex<Option<WindowId>>,
    /// Smallest size any window accepts, to simulate app minimums.
    pub min_size: Mutex<(f64, f64)>,
    sinks: Sinks,
    next_sink: Mutex<u64>,
}

impl FakeBackend {
    pub fn with_monitor(work: Rect) -> Arc<Self> {
        let b = Self::default();
        b.monitors.lock().unwrap().push(Monitor { id: 0, frame: work, work_area: work });
        Arc::new(b)
    }

    pub fn add_window(&self, id: WindowId, app: &str, rect: Rect) {
        self.windows.lock().unwrap().push(WindowInfo {
            id,
            pid: id as i32,
            app: app.into(),
            app_id: format!("com.test.{app}"),
            title: format!("{app} window"),
            rect,
        });
    }

    pub fn rect_of(&self, id: WindowId) -> Rect {
        self.windows.lock().unwrap().iter().find(|w| w.id == id).unwrap().rect
    }

    /// Simulate the user dragging or resizing a window.
    pub fn user_sets(&self, id: WindowId, rect: Rect) {
        self.apply(id, rect);
    }

    pub fn close(&self, id: WindowId) {
        self.windows.lock().unwrap().retain(|w| w.id != id);
        self.emit(WindowEvent::Gone(id));
    }

    fn apply(&self, id: WindowId, rect: Rect) {
        let (mw, mh) = *self.min_size.lock().unwrap();
        let rect = Rect::new(rect.x, rect.y, rect.w.max(mw), rect.h.max(mh));
        if let Some(w) = self.windows.lock().unwrap().iter_mut().find(|w| w.id == id) {
            w.rect = rect;
        }
        self.emit(WindowEvent::Geometry(id));
    }

    fn emit(&self, ev: WindowEvent) {
        let id = match ev {
            WindowEvent::Geometry(i) | WindowEvent::Gone(i) => i,
        };
        let sinks: Vec<EventSink> = self
            .sinks
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, w, _)| *w == id)
            .map(|(_, _, s)| s.clone())
            .collect();
        for s in sinks {
            s(ev);
        }
    }
}

struct Guard {
    key: u64,
    sinks: Sinks,
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.sinks.lock().unwrap().retain(|(k, _, _)| *k != self.key);
    }
}

impl WindowBackend for FakeBackend {
    fn is_trusted(&self) -> bool {
        true
    }
    fn request_trust(&self) {}

    fn frontmost_window(&self) -> Option<WindowInfo> {
        let id = (*self.frontmost.lock().unwrap())?;
        self.window(id)
    }

    fn list_windows(&self) -> Vec<WindowInfo> {
        self.windows.lock().unwrap().clone()
    }

    fn window(&self, id: WindowId) -> Option<WindowInfo> {
        self.windows.lock().unwrap().iter().find(|w| w.id == id).cloned()
    }

    fn set_rect(&self, id: WindowId, rect: Rect) -> Result<(), String> {
        self.window(id).ok_or("no such window")?;
        self.apply(id, rect);
        Ok(())
    }

    fn focus(&self, id: WindowId) -> Result<(), String> {
        *self.frontmost.lock().unwrap() = Some(id);
        Ok(())
    }

    fn minimize(&self, id: WindowId) -> Result<(), String> {
        self.windows.lock().unwrap().retain(|w| w.id != id);
        self.emit(WindowEvent::Gone(id));
        Ok(())
    }

    fn monitors(&self) -> Vec<Monitor> {
        self.monitors.lock().unwrap().clone()
    }

    fn launch(&self, app_id: &str) -> Result<(), String> {
        self.launched.lock().unwrap().push(app_id.to_string());
        Ok(())
    }

    fn watch(&self, id: WindowId, sink: EventSink) -> Result<Box<dyn Send>, String> {
        self.window(id).ok_or("no such window")?;
        let mut n = self.next_sink.lock().unwrap();
        *n += 1;
        let key = *n;
        self.sinks.lock().unwrap().push((key, id, sink));
        Ok(Box::new(Guard { key, sinks: self.sinks.clone() }))
    }
}
