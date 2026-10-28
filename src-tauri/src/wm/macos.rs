//! macOS backend: Accessibility API (AXUIElement) for control, CGWindowList for z-order,
//! AppKit for app identity and screen geometry.

use std::ffi::c_void;
use std::process::Command;

use core_foundation::base::{CFRelease, CFRetain, CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation_sys::dictionary::{CFDictionaryGetValue, CFDictionaryRef};
use core_foundation_sys::runloop::{
    CFRunLoopAddSource, CFRunLoopGetMain, CFRunLoopRemoveSource, CFRunLoopSourceRef,
    kCFRunLoopCommonModes,
};
use objc2_app_kit::{NSRunningApplication, NSScreen};
use objc2_foundation::MainThreadMarker;

use super::{EventSink, Monitor, Rect, WindowBackend, WindowEvent, WindowId, WindowInfo};

type AxRef = *const c_void;
type AxError = i32;
type ObserverRef = *mut c_void;

const AX_SUCCESS: AxError = 0;
const AX_VALUE_POINT: u32 = 1;
const AX_VALUE_SIZE: u32 = 2;

const CG_ON_SCREEN_ONLY: u32 = 1 << 0;
const CG_EXCLUDE_DESKTOP: u32 = 1 << 4;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Pt {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Sz {
    w: f64,
    h: f64,
}

type ObserverCallback =
    extern "C" fn(observer: ObserverRef, element: AxRef, notification: CFStringRef, refcon: *mut c_void);

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    fn AXUIElementCreateApplication(pid: i32) -> AxRef;
    fn AXUIElementCreateSystemWide() -> AxRef;
    fn AXUIElementCopyAttributeValue(el: AxRef, attr: CFStringRef, value: *mut CFTypeRef) -> AxError;
    fn AXUIElementSetAttributeValue(el: AxRef, attr: CFStringRef, value: CFTypeRef) -> AxError;
    fn AXUIElementPerformAction(el: AxRef, action: CFStringRef) -> AxError;
    fn AXUIElementGetPid(el: AxRef, pid: *mut i32) -> AxError;
    fn AXUIElementSetMessagingTimeout(el: AxRef, seconds: f32) -> AxError;
    fn AXValueCreate(kind: u32, ptr: *const c_void) -> CFTypeRef;
    fn AXValueGetValue(value: CFTypeRef, kind: u32, ptr: *mut c_void) -> bool;
    fn _AXUIElementGetWindow(el: AxRef, id: *mut u32) -> AxError;
    fn AXObserverCreate(pid: i32, cb: ObserverCallback, out: *mut ObserverRef) -> AxError;
    fn AXObserverAddNotification(obs: ObserverRef, el: AxRef, notif: CFStringRef, refcon: *mut c_void) -> AxError;
    fn AXObserverGetRunLoopSource(obs: ObserverRef) -> CFRunLoopSourceRef;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> CFArrayRef;
}

fn pack(pid: i32, number: u32) -> WindowId {
    ((pid as u64) << 32) | number as u64
}

fn unpack(id: WindowId) -> (i32, u32) {
    ((id >> 32) as i32, (id & 0xFFFF_FFFF) as u32)
}

/// Owned AXUIElement.
struct AxEl(AxRef);

unsafe impl Send for AxEl {}
unsafe impl Sync for AxEl {}

impl Drop for AxEl {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) }
    }
}

impl AxEl {
    fn app(pid: i32) -> Option<Self> {
        let r = unsafe { AXUIElementCreateApplication(pid) };
        if r.is_null() {
            return None;
        }
        // A hung app must not freeze the palette.
        unsafe { AXUIElementSetMessagingTimeout(r, 0.5) };
        Some(Self(r))
    }

    /// Copy an attribute; `None` when absent or unreadable.
    fn attr(&self, name: &str) -> Option<CFType> {
        let key = CFString::new(name);
        let mut out: CFTypeRef = std::ptr::null();
        let err = unsafe { AXUIElementCopyAttributeValue(self.0, key.as_concrete_TypeRef(), &mut out) };
        if err != AX_SUCCESS || out.is_null() {
            return None;
        }
        Some(unsafe { CFType::wrap_under_create_rule(out) })
    }

    fn element_attr(&self, name: &str) -> Option<AxEl> {
        let v = self.attr(name)?;
        let r = v.as_CFTypeRef();
        unsafe { CFRetain(r) };
        Some(AxEl(r))
    }

    fn string_attr(&self, name: &str) -> Option<String> {
        let v = self.attr(name)?;
        v.downcast_into::<CFString>().map(|s| s.to_string())
    }

    fn bool_attr(&self, name: &str) -> Option<bool> {
        let v = self.attr(name)?;
        v.downcast_into::<CFBoolean>().map(bool::from)
    }

    fn set_bool(&self, name: &str, value: bool) -> bool {
        let key = CFString::new(name);
        let v = if value { CFBoolean::true_value() } else { CFBoolean::false_value() };
        unsafe { AXUIElementSetAttributeValue(self.0, key.as_concrete_TypeRef(), v.as_CFTypeRef()) == AX_SUCCESS }
    }

    fn perform(&self, action: &str) -> bool {
        let a = CFString::new(action);
        unsafe { AXUIElementPerformAction(self.0, a.as_concrete_TypeRef()) == AX_SUCCESS }
    }

    fn windows(&self) -> Vec<AxEl> {
        let Some(v) = self.attr("AXWindows") else { return Vec::new() };
        let arr = v.as_CFTypeRef() as CFArrayRef;
        let n = unsafe { CFArrayGetCount(arr) };
        (0..n)
            .filter_map(|i| {
                let r = unsafe { CFArrayGetValueAtIndex(arr, i) };
                if r.is_null() {
                    return None;
                }
                unsafe { CFRetain(r) };
                Some(AxEl(r))
            })
            .collect()
    }

    fn position(&self) -> Option<Pt> {
        let v = self.attr("AXPosition")?;
        let mut p = Pt::default();
        let ok = unsafe { AXValueGetValue(v.as_CFTypeRef(), AX_VALUE_POINT, &mut p as *mut _ as *mut c_void) };
        ok.then_some(p)
    }

    fn size(&self) -> Option<Sz> {
        let v = self.attr("AXSize")?;
        let mut s = Sz::default();
        let ok = unsafe { AXValueGetValue(v.as_CFTypeRef(), AX_VALUE_SIZE, &mut s as *mut _ as *mut c_void) };
        ok.then_some(s)
    }

    fn rect(&self) -> Option<Rect> {
        let (p, s) = (self.position()?, self.size()?);
        Some(Rect::new(p.x, p.y, s.w, s.h))
    }

    fn set_position(&self, x: f64, y: f64) -> bool {
        let p = Pt { x, y };
        let v = unsafe { AXValueCreate(AX_VALUE_POINT, &p as *const _ as *const c_void) };
        if v.is_null() {
            return false;
        }
        let key = CFString::new("AXPosition");
        let ok = unsafe { AXUIElementSetAttributeValue(self.0, key.as_concrete_TypeRef(), v) == AX_SUCCESS };
        unsafe { CFRelease(v) };
        ok
    }

    fn set_size(&self, w: f64, h: f64) -> bool {
        let s = Sz { w, h };
        let v = unsafe { AXValueCreate(AX_VALUE_SIZE, &s as *const _ as *const c_void) };
        if v.is_null() {
            return false;
        }
        let key = CFString::new("AXSize");
        let ok = unsafe { AXUIElementSetAttributeValue(self.0, key.as_concrete_TypeRef(), v) == AX_SUCCESS };
        unsafe { CFRelease(v) };
        ok
    }

    fn window_number(&self) -> Option<u32> {
        let mut n: u32 = 0;
        (unsafe { _AXUIElementGetWindow(self.0, &mut n) } == AX_SUCCESS).then_some(n)
    }

    fn pid(&self) -> Option<i32> {
        let mut p: i32 = 0;
        (unsafe { AXUIElementGetPid(self.0, &mut p) } == AX_SUCCESS).then_some(p)
    }

    fn is_standard_window(&self) -> bool {
        self.string_attr("AXRole").as_deref() == Some("AXWindow")
            && self.string_attr("AXSubrole").as_deref() == Some("AXStandardWindow")
    }
}

/// `(pid, window number)` of every normal on-screen window, front to back.
fn z_order() -> Vec<(i32, u32)> {
    let list = unsafe { CGWindowListCopyWindowInfo(CG_ON_SCREEN_ONLY | CG_EXCLUDE_DESKTOP, 0) };
    if list.is_null() {
        return Vec::new();
    }
    let number_key = CFString::new("kCGWindowNumber");
    let layer_key = CFString::new("kCGWindowLayer");
    let pid_key = CFString::new("kCGWindowOwnerPID");
    let get_i64 = |d: CFDictionaryRef, k: &CFString| -> Option<i64> {
        let v = unsafe { CFDictionaryGetValue(d, k.as_concrete_TypeRef() as *const c_void) };
        if v.is_null() {
            return None;
        }
        unsafe { CFNumber::wrap_under_get_rule(v as _) }.to_i64()
    };
    let n = unsafe { CFArrayGetCount(list) };
    let mut out = Vec::new();
    for i in 0..n {
        let d = unsafe { CFArrayGetValueAtIndex(list, i) } as CFDictionaryRef;
        if d.is_null() || get_i64(d, &layer_key) != Some(0) {
            continue;
        }
        if let (Some(num), Some(pid)) = (get_i64(d, &number_key), get_i64(d, &pid_key)) {
            out.push((pid as i32, num as u32));
        }
    }
    unsafe { CFRelease(list as CFTypeRef) };
    out
}

fn app_identity(pid: i32) -> (String, String) {
    let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid);
    let name = app
        .as_ref()
        .and_then(|a| a.localizedName())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let bundle = app
        .as_ref()
        .and_then(|a| a.bundleIdentifier())
        .map(|s| s.to_string())
        .unwrap_or_else(|| name.clone());
    (name, bundle)
}

fn info_for(win: &AxEl, pid: i32, number: u32) -> Option<WindowInfo> {
    let (app, app_id) = app_identity(pid);
    Some(WindowInfo {
        id: pack(pid, number),
        pid,
        app,
        app_id,
        title: win.string_attr("AXTitle").unwrap_or_default(),
        rect: win.rect()?,
    })
}

fn find_window(id: WindowId) -> Option<(AxEl, i32, u32)> {
    let (pid, number) = unpack(id);
    let app = AxEl::app(pid)?;
    let win = app.windows().into_iter().find(|w| w.window_number() == Some(number))?;
    Some((win, pid, number))
}

pub struct MacBackend;

impl MacBackend {
    pub fn new() -> Self {
        Self
    }
}

impl WindowBackend for MacBackend {
    fn is_trusted(&self) -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    fn request_trust(&self) {
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let opts = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
        unsafe { AXIsProcessTrustedWithOptions(opts.as_concrete_TypeRef()) };
    }

    fn frontmost_window(&self) -> Option<WindowInfo> {
        let sys = AxEl(unsafe { AXUIElementCreateSystemWide() });
        let app = sys.element_attr("AXFocusedApplication")?;
        let pid = app.pid()?;
        if pid as u32 == std::process::id() {
            return None;
        }
        let win = app
            .element_attr("AXFocusedWindow")
            .or_else(|| app.element_attr("AXMainWindow"))?;
        info_for(&win, pid, win.window_number()?)
    }

    fn list_windows(&self) -> Vec<WindowInfo> {
        let own = std::process::id() as i32;
        let order = z_order();
        let mut by_pid: Vec<(i32, Vec<AxEl>)> = Vec::new();
        let mut out = Vec::new();
        for (pid, number) in order {
            if pid == own {
                continue;
            }
            if !by_pid.iter().any(|(p, _)| *p == pid) {
                let wins = AxEl::app(pid).map(|a| a.windows()).unwrap_or_default();
                by_pid.push((pid, wins));
            }
            let wins = &by_pid.iter().find(|(p, _)| *p == pid).unwrap().1;
            let Some(win) = wins.iter().find(|w| w.window_number() == Some(number)) else { continue };
            if !win.is_standard_window() || win.bool_attr("AXMinimized") == Some(true) {
                continue;
            }
            if let Some(info) = info_for(win, pid, number) {
                out.push(info);
            }
        }
        out
    }

    fn window(&self, id: WindowId) -> Option<WindowInfo> {
        let (win, pid, number) = find_window(id)?;
        info_for(&win, pid, number)
    }

    fn set_rect(&self, id: WindowId, rect: Rect) -> Result<(), String> {
        let (win, _, _) = find_window(id).ok_or("window not found")?;
        // Size first, then position, then size again: apps clamp size against the
        // current position, so a single pass can leave the window short.
        win.set_size(rect.w, rect.h);
        if !win.set_position(rect.x, rect.y) {
            return Err("could not move window".into());
        }
        win.set_size(rect.w, rect.h);
        Ok(())
    }

    fn focus(&self, id: WindowId) -> Result<(), String> {
        let (win, pid, _) = find_window(id).ok_or("window not found")?;
        win.perform("AXRaise");
        if let Some(app) = AxEl::app(pid) {
            app.set_bool("AXFrontmost", true);
        }
        Ok(())
    }

    fn minimize(&self, id: WindowId) -> Result<(), String> {
        let (win, _, _) = find_window(id).ok_or("window not found")?;
        if win.set_bool("AXMinimized", true) { Ok(()) } else { Err("could not minimize".into()) }
    }

    fn monitors(&self) -> Vec<Monitor> {
        // SAFETY: only reads immutable screen geometry; callers run on Tauri's main/async
        // threads and AppKit tolerates these getters off the main thread.
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        let screens = NSScreen::screens(mtm);
        let Some(primary) = screens.iter().next() else { return Vec::new() };
        let primary_h = primary.frame().size.height;
        let to_top_left = |x: f64, y: f64, w: f64, h: f64| Rect::new(x, primary_h - (y + h), w, h);
        screens
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let f = s.frame();
                let v = s.visibleFrame();
                Monitor {
                    id: i as u32,
                    frame: to_top_left(f.origin.x, f.origin.y, f.size.width, f.size.height),
                    work_area: to_top_left(v.origin.x, v.origin.y, v.size.width, v.size.height),
                }
            })
            .collect()
    }

    fn launch(&self, app_id: &str) -> Result<(), String> {
        let status = if app_id.contains('.') {
            Command::new("open").args(["-b", app_id]).status()
        } else {
            Command::new("open").args(["-a", app_id]).status()
        };
        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(_) => Err(format!("could not launch {app_id}")),
            Err(e) => Err(e.to_string()),
        }
    }

    fn watch(&self, id: WindowId, sink: EventSink) -> Result<Box<dyn Send>, String> {
        let (win, pid, _) = find_window(id).ok_or("window not found")?;
        let mut observer: ObserverRef = std::ptr::null_mut();
        if unsafe { AXObserverCreate(pid, observer_cb, &mut observer) } != AX_SUCCESS || observer.is_null() {
            return Err("could not create observer".into());
        }
        let ctx = Box::into_raw(Box::new(WatchCtx { sink, id }));
        for n in ["AXMoved", "AXResized", "AXUIElementDestroyed", "AXWindowMiniaturized"] {
            let name = CFString::new(n);
            unsafe { AXObserverAddNotification(observer, win.0, name.as_concrete_TypeRef(), ctx as *mut c_void) };
        }
        let source = unsafe { AXObserverGetRunLoopSource(observer) };
        unsafe { CFRunLoopAddSource(CFRunLoopGetMain(), source, kCFRunLoopCommonModes) };
        Ok(Box::new(WatchGuard { observer, source, ctx, _win: win }))
    }
}

struct WatchCtx {
    sink: EventSink,
    id: WindowId,
}

extern "C" fn observer_cb(_obs: ObserverRef, _el: AxRef, notification: CFStringRef, refcon: *mut c_void) {
    if refcon.is_null() {
        return;
    }
    let ctx = unsafe { &*(refcon as *const WatchCtx) };
    let name = unsafe { CFString::wrap_under_get_rule(notification) }.to_string();
    let ev = match name.as_str() {
        "AXMoved" | "AXResized" => WindowEvent::Geometry(ctx.id),
        _ => WindowEvent::Gone(ctx.id),
    };
    (ctx.sink)(ev);
}

struct WatchGuard {
    observer: ObserverRef,
    source: CFRunLoopSourceRef,
    ctx: *mut WatchCtx,
    _win: AxEl,
}

unsafe impl Send for WatchGuard {}

impl Drop for WatchGuard {
    fn drop(&mut self) {
        unsafe {
            CFRunLoopRemoveSource(CFRunLoopGetMain(), self.source, kCFRunLoopCommonModes);
            CFRelease(self.observer as CFTypeRef);
            drop(Box::from_raw(self.ctx));
        }
    }
}

#[cfg(test)]
mod live {
    use super::*;

    /// Needs Accessibility permission for the test host; run with `--ignored`.
    #[test]
    #[ignore]
    fn lists_real_windows_and_monitors() {
        let b = MacBackend::new();
        println!("trusted: {}", b.is_trusted());
        let monitors = b.monitors();
        println!("monitors: {}", monitors.len());
        for m in &monitors {
            println!("  #{} frame {:?} work {:?}", m.id, (m.frame.x, m.frame.y, m.frame.w, m.frame.h), (m.work_area.x, m.work_area.y, m.work_area.w, m.work_area.h));
        }
        let wins = b.list_windows();
        println!("windows: {}", wins.len());
        for w in wins.iter().take(8) {
            println!("  {} (pid {}) {:?}", w.app, w.pid, (w.rect.x, w.rect.y, w.rect.w, w.rect.h));
        }
        println!("frontmost app: {:?}", b.frontmost_window().map(|w| w.app));
        assert!(!monitors.is_empty());
    }
}
