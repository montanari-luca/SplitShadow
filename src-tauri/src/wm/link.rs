//! Linked split: two windows that share an edge and resize together.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::layouts::{self, Layout};
use super::{EventSink, Rect, WindowBackend, WindowEvent, WindowId, monitor_for};

/// Smallest size either side of the divider may be squeezed to.
const MIN_PANE: f64 = 150.0;
/// A window moved this far without changing size is being dragged away: unlink.
const DRAG_THRESHOLD: f64 = 12.0;
/// Rect differences at or below this are rounding noise, not user input.
const NOISE: f64 = 1.5;
/// Events are coalesced over this window so a drag costs one update per frame.
const COALESCE: Duration = Duration::from_millis(14);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Side by side, divider is vertical.
    Side,
    /// Stacked, divider is horizontal.
    Stack,
}

/// (main position, main length, cross position, cross length) of a rect along `axis`.
fn split_axes(axis: Axis, r: &Rect) -> (f64, f64, f64, f64) {
    match axis {
        Axis::Side => (r.x, r.w, r.y, r.h),
        Axis::Stack => (r.y, r.h, r.x, r.w),
    }
}

fn join_axes(axis: Axis, mp: f64, ml: f64, cp: f64, cl: f64) -> Rect {
    match axis {
        Axis::Side => Rect::new(mp, cp, ml, cl),
        Axis::Stack => Rect::new(cp, mp, cl, ml),
    }
}

/// Where the other window goes after `driver` changed, plus the updated shared span.
/// The pair always fills `span` along the main axis and shares the driver's cross extent.
pub fn follow(axis: Axis, driver_first: bool, driver: &Rect, span: &Rect) -> (Rect, Rect) {
    let (dmp, dml, dcp, dcl) = split_axes(axis, driver);
    let (smp, sml, _, _) = split_axes(axis, span);
    let span_end = smp + sml;

    let (other_mp, other_ml, new_span_mp, new_span_ml) = if driver_first {
        let lo = dmp + MIN_PANE;
        let hi = (span_end - MIN_PANE).max(lo);
        let edge = (dmp + dml).clamp(lo, hi);
        (edge, span_end - edge, dmp, span_end - dmp)
    } else {
        let hi = dmp + dml - MIN_PANE;
        let lo = (smp + MIN_PANE).min(hi);
        let edge = dmp.clamp(lo, hi);
        (smp, edge - smp, smp, dmp + dml - smp)
    };
    (
        join_axes(axis, other_mp, other_ml, dcp, dcl),
        join_axes(axis, new_span_mp, new_span_ml, dcp, dcl),
    )
}

struct Active {
    a: WindowId,
    b: WindowId,
    axis: Axis,
    a_first: bool,
    span: Rect,
    last_a: Rect,
    last_b: Rect,
    _guards: Vec<Box<dyn Send>>,
}

pub struct LinkManager {
    backend: Arc<dyn WindowBackend>,
    active: Arc<Mutex<Option<Active>>>,
    tx: Sender<WindowEvent>,
    rx: Mutex<Receiver<WindowEvent>>,
}

impl LinkManager {
    /// `spawn_worker` is false in tests, which drive `pump` by hand.
    pub fn new(backend: Arc<dyn WindowBackend>, spawn_worker: bool) -> Arc<Self> {
        let (tx, rx) = channel();
        let m = Arc::new(Self { backend, active: Arc::new(Mutex::new(None)), tx, rx: Mutex::new(rx) });
        if spawn_worker {
            let w = Arc::downgrade(&m);
            thread::spawn(move || {
                loop {
                    let Some(m) = w.upgrade() else { break };
                    if !m.pump_blocking() {
                        break;
                    }
                }
            });
        }
        m
    }

    /// Tile `a` and `b` side by side (or stacked) in the work area of `a`'s display and link them.
    pub fn split(&self, a: WindowId, b: WindowId, axis: Axis, almost_max: f64) -> Result<(), String> {
        let win_a = self.backend.window(a).ok_or("The selected window is gone.")?;
        self.backend.window(b).ok_or("That window is gone.")?;
        let monitors = self.backend.monitors();
        let work = monitor_for(&monitors, &win_a.rect).ok_or("No display found.")?.work_area;
        let (la, lb) = match axis {
            Axis::Side => (Layout::LeftHalf, Layout::RightHalf),
            Axis::Stack => (Layout::TopHalf, Layout::BottomHalf),
        };
        self.backend.set_rect(a, layouts::compute(la, &work, &win_a.rect, almost_max))?;
        self.backend.set_rect(b, layouts::compute(lb, &work, &win_a.rect, almost_max))?;
        self.link(a, b, axis, work)
    }

    /// Link two windows that are already arranged (used when restoring a workspace).
    pub fn link_existing(&self, a: WindowId, b: WindowId) -> Result<(), String> {
        let ra = self.backend.window(a).ok_or("window missing")?.rect;
        let rb = self.backend.window(b).ok_or("window missing")?.rect;
        let (ca, cb) = (ra.center(), rb.center());
        let axis = if (ca.0 - cb.0).abs() >= (ca.1 - cb.1).abs() { Axis::Side } else { Axis::Stack };
        let span = bounding(&ra, &rb);
        self.link(a, b, axis, span)
    }

    fn link(&self, a: WindowId, b: WindowId, axis: Axis, span: Rect) -> Result<(), String> {
        self.unlink();
        let ra = self.backend.window(a).ok_or("window missing")?.rect;
        let rb = self.backend.window(b).ok_or("window missing")?.rect;
        let a_first = split_axes(axis, &ra).0 <= split_axes(axis, &rb).0;
        let sink: EventSink = {
            let tx = self.tx.clone();
            Arc::new(move |ev| {
                let _ = tx.send(ev);
            })
        };
        let guards = vec![self.backend.watch(a, sink.clone())?, self.backend.watch(b, sink)?];
        *self.active.lock().unwrap() = Some(Active { a, b, axis, a_first, span, last_a: ra, last_b: rb, _guards: guards });
        Ok(())
    }

    pub fn unlink(&self) {
        // Dropping the guards unsubscribes. Done outside the lock to keep it short.
        let old = self.active.lock().unwrap().take();
        drop(old);
    }

    pub fn pair(&self) -> Option<(WindowId, WindowId)> {
        self.active.lock().unwrap().as_ref().map(|l| (l.a, l.b))
    }

    pub fn is_linked(&self, id: WindowId) -> bool {
        self.pair().map(|(a, b)| a == id || b == id).unwrap_or(false)
    }

    fn pump_blocking(&self) -> bool {
        let first = match self.rx.lock().unwrap().recv() {
            Ok(ev) => ev,
            Err(_) => return false,
        };
        thread::sleep(COALESCE);
        self.process(first);
        true
    }

    /// Handle everything queued so far (non-blocking). Used by tests.
    #[cfg(test)]
    pub fn pump(&self) {
        let first = self.rx.lock().unwrap().try_recv().ok();
        if let Some(ev) = first {
            self.process(ev);
        }
    }

    fn process(&self, first: WindowEvent) {
        let mut gone = matches!(first, WindowEvent::Gone(_));
        {
            let rx = self.rx.lock().unwrap();
            while let Ok(ev) = rx.try_recv() {
                gone |= matches!(ev, WindowEvent::Gone(_));
            }
        }
        if gone {
            self.unlink();
            return;
        }
        let mut guard = self.active.lock().unwrap();
        let Some(link) = guard.as_mut() else { return };
        let (Some(wa), Some(wb)) = (self.backend.window(link.a), self.backend.window(link.b)) else {
            drop(guard);
            self.unlink();
            return;
        };
        let (cur_a, cur_b) = (wa.rect, wb.rect);
        let changed_a = !cur_a.approx_eq(&link.last_a, NOISE);
        let changed_b = !cur_b.approx_eq(&link.last_b, NOISE);
        let (driver_is_a, driver, prev) = match (changed_a, changed_b) {
            (false, false) => return, // echo of our own move
            (true, _) => (true, cur_a, link.last_a),
            (false, true) => (false, cur_b, link.last_b),
        };

        let same_size = (driver.w - prev.w).abs() <= 2.0 && (driver.h - prev.h).abs() <= 2.0;
        let moved = (driver.x - prev.x).abs().max((driver.y - prev.y).abs());
        if same_size && moved > DRAG_THRESHOLD {
            drop(guard);
            self.unlink();
            return;
        }

        let driver_first = if driver_is_a { link.a_first } else { !link.a_first };
        let (other_target, span) = follow(link.axis, driver_first, &driver, &link.span);
        let other_id = if driver_is_a { link.b } else { link.a };
        let _ = self.backend.set_rect(other_id, other_target);
        // Read back: apps may clamp, and this is what makes the echo event a no-op.
        let other_actual = self.backend.window(other_id).map(|w| w.rect).unwrap_or(other_target);
        link.span = span;
        if driver_is_a {
            link.last_a = driver;
            link.last_b = other_actual;
        } else {
            link.last_b = driver;
            link.last_a = other_actual;
        }
    }
}

fn bounding(a: &Rect, b: &Rect) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    Rect::new(x, y, a.right().max(b.right()) - x, a.bottom().max(b.bottom()) - y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wm::fake::FakeBackend;

    const WORK: Rect = Rect::new(0.0, 25.0, 1200.0, 800.0);

    #[test]
    fn follow_left_driver_widens() {
        let (o, span) = follow(Axis::Side, true, &Rect::new(0.0, 25.0, 700.0, 800.0), &WORK);
        assert_eq!(o, Rect::new(700.0, 25.0, 500.0, 800.0));
        assert_eq!(span, WORK);
    }

    #[test]
    fn follow_right_driver_widens() {
        let (o, _) = follow(Axis::Side, false, &Rect::new(500.0, 25.0, 700.0, 800.0), &WORK);
        assert_eq!(o, Rect::new(0.0, 25.0, 500.0, 800.0));
    }

    #[test]
    fn follow_stacked() {
        let (o, _) = follow(Axis::Stack, true, &Rect::new(0.0, 25.0, 1200.0, 300.0), &WORK);
        assert_eq!(o, Rect::new(0.0, 325.0, 1200.0, 500.0));
    }

    #[test]
    fn follow_never_squeezes_below_minimum() {
        let (o, _) = follow(Axis::Side, true, &Rect::new(0.0, 25.0, 1190.0, 800.0), &WORK);
        assert_eq!(o.w, MIN_PANE);
        assert_eq!(o.right(), WORK.right());
    }

    #[test]
    fn follow_shares_cross_extent() {
        let (o, span) = follow(Axis::Side, true, &Rect::new(0.0, 100.0, 600.0, 500.0), &WORK);
        assert_eq!((o.y, o.h), (100.0, 500.0));
        assert_eq!((span.y, span.h), (100.0, 500.0));
    }

    fn setup() -> (Arc<FakeBackend>, Arc<LinkManager>) {
        let b = FakeBackend::with_monitor(WORK);
        b.add_window(1, "Notes", Rect::new(50.0, 100.0, 300.0, 300.0));
        b.add_window(2, "Slack", Rect::new(400.0, 100.0, 300.0, 300.0));
        let m = LinkManager::new(b.clone(), false);
        (b, m)
    }

    fn settle(m: &LinkManager) {
        // Echo events from our own moves arrive after the user's; drain them all.
        for _ in 0..6 {
            m.pump();
        }
    }

    #[test]
    fn split_tiles_and_links() {
        let (b, m) = setup();
        m.split(1, 2, Axis::Side, 0.9).unwrap();
        assert_eq!(b.rect_of(1), Rect::new(0.0, 25.0, 600.0, 800.0));
        assert_eq!(b.rect_of(2), Rect::new(600.0, 25.0, 600.0, 800.0));
        assert_eq!(m.pair(), Some((1, 2)));
        settle(&m);
        // Our own tiling must not look like user input.
        assert_eq!(b.rect_of(2), Rect::new(600.0, 25.0, 600.0, 800.0));
        assert!(m.pair().is_some());
    }

    #[test]
    fn resizing_one_window_moves_the_other() {
        let (b, m) = setup();
        m.split(1, 2, Axis::Side, 0.9).unwrap();
        settle(&m);
        b.user_sets(1, Rect::new(0.0, 25.0, 800.0, 800.0));
        settle(&m);
        assert_eq!(b.rect_of(2), Rect::new(800.0, 25.0, 400.0, 800.0));
        // And back from the other side.
        b.user_sets(2, Rect::new(300.0, 25.0, 900.0, 800.0));
        settle(&m);
        assert_eq!(b.rect_of(1), Rect::new(0.0, 25.0, 300.0, 800.0));
        assert!(m.pair().is_some());
    }

    #[test]
    fn dragging_a_window_away_unlinks() {
        let (b, m) = setup();
        m.split(1, 2, Axis::Side, 0.9).unwrap();
        settle(&m);
        b.user_sets(1, Rect::new(200.0, 300.0, 600.0, 800.0));
        settle(&m);
        assert!(m.pair().is_none());
        assert_eq!(b.rect_of(2), Rect::new(600.0, 25.0, 600.0, 800.0));
    }

    #[test]
    fn closing_a_window_unlinks() {
        let (b, m) = setup();
        m.split(1, 2, Axis::Side, 0.9).unwrap();
        b.close(2);
        settle(&m);
        assert!(m.pair().is_none());
    }

    #[test]
    fn app_minimum_size_does_not_cause_a_fight() {
        let (b, m) = setup();
        m.split(1, 2, Axis::Side, 0.9).unwrap();
        settle(&m);
        *b.min_size.lock().unwrap() = (450.0, 0.0);
        b.user_sets(1, Rect::new(0.0, 25.0, 900.0, 800.0));
        settle(&m);
        assert_eq!(b.rect_of(2).w, 450.0);
        let after = b.rect_of(1);
        settle(&m);
        assert_eq!(b.rect_of(1), after, "no further adjustments");
    }

    #[test]
    fn unlink_stops_following() {
        let (b, m) = setup();
        m.split(1, 2, Axis::Side, 0.9).unwrap();
        settle(&m);
        m.unlink();
        b.user_sets(1, Rect::new(0.0, 25.0, 800.0, 800.0));
        settle(&m);
        assert_eq!(b.rect_of(2), Rect::new(600.0, 25.0, 600.0, 800.0));
    }

    #[test]
    fn link_existing_detects_axis() {
        let (b, m) = setup();
        b.set_rect(1, Rect::new(0.0, 25.0, 1200.0, 400.0)).unwrap();
        b.set_rect(2, Rect::new(0.0, 425.0, 1200.0, 400.0)).unwrap();
        m.link_existing(1, 2).unwrap();
        settle(&m);
        b.user_sets(1, Rect::new(0.0, 25.0, 1200.0, 500.0));
        settle(&m);
        assert_eq!(b.rect_of(2), Rect::new(0.0, 525.0, 1200.0, 300.0));
    }
}
