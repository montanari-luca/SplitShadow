//! Pure geometry for every snap layout. No platform code, fully unit tested.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    pub fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// Area of the intersection with another rect, 0 when disjoint.
    pub fn overlap_area(&self, other: &Rect) -> f64 {
        let w = self.right().min(other.right()) - self.x.max(other.x);
        let h = self.bottom().min(other.bottom()) - self.y.max(other.y);
        if w > 0.0 && h > 0.0 { w * h } else { 0.0 }
    }

    pub fn approx_eq(&self, other: &Rect, tol: f64) -> bool {
        (self.x - other.x).abs() <= tol
            && (self.y - other.y).abs() <= tol
            && (self.w - other.w).abs() <= tol
            && (self.h - other.h).abs() <= tol
    }

    /// This rect expressed as fractions of `area`.
    pub fn to_fractions(self, area: &Rect) -> Rect {
        Rect::new(
            (self.x - area.x) / area.w,
            (self.y - area.y) / area.h,
            self.w / area.w,
            self.h / area.h,
        )
    }

    /// Resolve fractions of `area` back to absolute coordinates.
    pub fn scaled_into(self, area: &Rect) -> Rect {
        Rect::new(
            area.x + self.x * area.w,
            area.y + self.y * area.h,
            self.w * area.w,
            self.h * area.h,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Layout {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    LeftThird,
    CenterThird,
    RightThird,
    LeftTwoThirds,
    RightTwoThirds,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Maximize,
    AlmostMaximize,
    Center,
}

/// Target rect for `layout` inside `work`. `current` is only used by `Center`
/// (keeps the size) and `pct` (0.5..=1.0) only by `AlmostMaximize`.
pub fn compute(layout: Layout, work: &Rect, current: &Rect, pct: f64) -> Rect {
    let Rect { x, y, w, h } = *work;
    let (hw, hh) = (w / 2.0, h / 2.0);
    let third = w / 3.0;
    match layout {
        Layout::LeftHalf => Rect::new(x, y, hw, h),
        Layout::RightHalf => Rect::new(x + hw, y, hw, h),
        Layout::TopHalf => Rect::new(x, y, w, hh),
        Layout::BottomHalf => Rect::new(x, y + hh, w, hh),
        Layout::LeftThird => Rect::new(x, y, third, h),
        Layout::CenterThird => Rect::new(x + third, y, third, h),
        Layout::RightThird => Rect::new(x + 2.0 * third, y, third, h),
        Layout::LeftTwoThirds => Rect::new(x, y, 2.0 * third, h),
        Layout::RightTwoThirds => Rect::new(x + third, y, 2.0 * third, h),
        Layout::TopLeft => Rect::new(x, y, hw, hh),
        Layout::TopRight => Rect::new(x + hw, y, hw, hh),
        Layout::BottomLeft => Rect::new(x, y + hh, hw, hh),
        Layout::BottomRight => Rect::new(x + hw, y + hh, hw, hh),
        Layout::Maximize => *work,
        Layout::AlmostMaximize => {
            let p = pct.clamp(0.5, 1.0);
            let (nw, nh) = (w * p, h * p);
            Rect::new(x + (w - nw) / 2.0, y + (h - nh) / 2.0, nw, nh)
        }
        Layout::Center => {
            let nw = current.w.min(w);
            let nh = current.h.min(h);
            Rect::new(x + (w - nw) / 2.0, y + (h - nh) / 2.0, nw, nh)
        }
    }
}

/// Grow (positive `step`) or shrink (negative) around the center, clamped to `work`.
pub fn resize_about_center(current: &Rect, work: &Rect, step: f64) -> Rect {
    let nw = (current.w * (1.0 + step)).clamp(120.0, work.w);
    let nh = (current.h * (1.0 + step)).clamp(80.0, work.h);
    let (cx, cy) = current.center();
    let x = (cx - nw / 2.0).clamp(work.x, (work.right() - nw).max(work.x));
    let y = (cy - nh / 2.0).clamp(work.y, (work.bottom() - nh).max(work.y));
    Rect::new(x, y, nw, nh)
}

/// Move `rect` onto another monitor keeping its relative position and size.
pub fn translate_between(rect: &Rect, from: &Rect, to: &Rect) -> Rect {
    rect.to_fractions(from).scaled_into(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = Rect::new(0.0, 25.0, 1200.0, 775.0);
    const CUR: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);

    #[test]
    fn halves_tile_the_work_area() {
        let l = compute(Layout::LeftHalf, &WORK, &CUR, 0.9);
        let r = compute(Layout::RightHalf, &WORK, &CUR, 0.9);
        assert_eq!(l, Rect::new(0.0, 25.0, 600.0, 775.0));
        assert_eq!(r, Rect::new(600.0, 25.0, 600.0, 775.0));
        assert_eq!(l.right(), r.x);
    }

    #[test]
    fn thirds_and_two_thirds() {
        let c = compute(Layout::CenterThird, &WORK, &CUR, 0.9);
        assert_eq!(c, Rect::new(400.0, 25.0, 400.0, 775.0));
        let t = compute(Layout::LeftTwoThirds, &WORK, &CUR, 0.9);
        assert_eq!(t, Rect::new(0.0, 25.0, 800.0, 775.0));
        let rt = compute(Layout::RightTwoThirds, &WORK, &CUR, 0.9);
        assert_eq!(rt.x, 400.0);
        assert_eq!(rt.right(), WORK.right());
    }

    #[test]
    fn quarters() {
        let br = compute(Layout::BottomRight, &WORK, &CUR, 0.9);
        assert_eq!(br, Rect::new(600.0, 412.5, 600.0, 387.5));
    }

    #[test]
    fn almost_maximize_is_centered_ninety_percent() {
        let r = compute(Layout::AlmostMaximize, &WORK, &CUR, 0.9);
        assert!((r.w - 1080.0).abs() < 1e-9);
        assert!((r.h - 697.5).abs() < 1e-9);
        assert!((r.x - 60.0).abs() < 1e-9);
        assert!((r.y - (25.0 + 38.75)).abs() < 1e-9);
    }

    #[test]
    fn almost_maximize_clamps_percentage() {
        let r = compute(Layout::AlmostMaximize, &WORK, &CUR, 5.0);
        assert_eq!(r, WORK);
    }

    #[test]
    fn center_keeps_size() {
        let r = compute(Layout::Center, &WORK, &CUR, 0.9);
        assert_eq!((r.w, r.h), (400.0, 300.0));
        assert_eq!(r.center(), WORK.center());
    }

    #[test]
    fn center_clamps_oversized_window() {
        let big = Rect::new(0.0, 0.0, 5000.0, 5000.0);
        let r = compute(Layout::Center, &WORK, &big, 0.9);
        assert_eq!(r, WORK);
    }

    #[test]
    fn resize_about_center_grows_and_stays_inside() {
        let r = resize_about_center(&CUR, &WORK, 0.1);
        assert!((r.w - 440.0).abs() < 1e-9);
        assert!(r.x >= WORK.x && r.right() <= WORK.right());
        let big = resize_about_center(&WORK, &WORK, 0.1);
        assert_eq!(big, WORK);
    }

    #[test]
    fn translate_keeps_relative_geometry() {
        let from = Rect::new(0.0, 0.0, 1000.0, 500.0);
        let to = Rect::new(1000.0, 0.0, 2000.0, 1000.0);
        let r = translate_between(&Rect::new(500.0, 250.0, 250.0, 125.0), &from, &to);
        assert_eq!(r, Rect::new(2000.0, 500.0, 500.0, 250.0));
    }

    #[test]
    fn fraction_round_trip() {
        let area = Rect::new(10.0, 20.0, 800.0, 600.0);
        let r = Rect::new(210.0, 170.0, 400.0, 300.0);
        assert!(r.to_fractions(&area).scaled_into(&area).approx_eq(&r, 1e-9));
    }

    #[test]
    fn layout_serde_is_kebab_case() {
        let s = serde_json::to_string(&Layout::LeftTwoThirds).unwrap();
        assert_eq!(s, "\"left-two-thirds\"");
        let l: Layout = serde_json::from_str("\"top-left\"").unwrap();
        assert_eq!(l, Layout::TopLeft);
    }
}
