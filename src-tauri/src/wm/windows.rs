//! Windows backend: Win32 for enumeration, placement and focus, WinEvent hooks for observing.

use std::ffi::c_void;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;

use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
use windows::core::BOOL;
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EVENT_OBJECT_DESTROY, EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_MINIMIZESTART, EnumWindows,
    GA_ROOT, GW_OWNER, GWL_EXSTYLE, GetAncestor, GetForegroundWindow, GetMessageW, GetWindow, GetWindowLongPtrW,
    GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, IsZoomed, MSG,
    OBJID_WINDOW, SW_MINIMIZE, SW_RESTORE, SWP_NOACTIVATE, SWP_NOZORDER, SetForegroundWindow, SetWindowPos,
    ShowWindow, TranslateMessage, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

use super::{EventSink, Monitor, Rect, WindowBackend, WindowEvent, WindowId, WindowInfo};

fn hwnd(id: WindowId) -> HWND {
    HWND(id as usize as *mut c_void)
}

fn id_of(h: HWND) -> WindowId {
    h.0 as usize as u64
}

fn to_rect(r: &RECT) -> Rect {
    Rect::new(r.left as f64, r.top as f64, (r.right - r.left) as f64, (r.bottom - r.top) as f64)
}

fn title_of(h: HWND) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(h, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

fn pid_of(h: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
    pid
}

fn exe_path(pid: u32) -> Option<String> {
    unsafe {
        let proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(proc, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(proc);
        ok.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

fn is_cloaked(h: HWND) -> bool {
    let mut cloaked = 0u32;
    let r = unsafe { DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut c_void, 4) };
    r.is_ok() && cloaked != 0
}

/// Visible bounds, without the invisible resize borders Windows adds around frames.
fn visible_rect(h: HWND) -> Option<RECT> {
    let mut r = RECT::default();
    let ok = unsafe {
        DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut _ as *mut c_void, std::mem::size_of::<RECT>() as u32)
    };
    if ok.is_ok() {
        return Some(r);
    }
    let mut w = RECT::default();
    unsafe { GetWindowRect(h, &mut w) }.ok()?;
    Some(w)
}

/// Windows a person would call "an app window": visible, titled, not a tool/popup.
fn is_app_window(h: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(h).as_bool() || IsIconic(h).as_bool() || is_cloaked(h) {
            return false;
        }
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
        if ex & WS_EX_TOOLWINDOW.0 != 0 {
            return false;
        }
        let owned = GetWindow(h, GW_OWNER).map(|o| !o.0.is_null()).unwrap_or(false);
        if owned && ex & WS_EX_APPWINDOW.0 == 0 {
            return false;
        }
        if title_of(h).is_empty() {
            return false;
        }
    }
    visible_rect(h).map(|r| r.right > r.left && r.bottom > r.top).unwrap_or(false)
}

fn info(h: HWND) -> Option<WindowInfo> {
    let rect = to_rect(&visible_rect(h)?);
    let pid = pid_of(h);
    let path = exe_path(pid).unwrap_or_default();
    let app = Path::new(&path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".into());
    Some(WindowInfo { id: id_of(h), pid: pid as i32, app, app_id: path, title: title_of(h), rect })
}

unsafe extern "system" fn collect_window(h: HWND, lparam: LPARAM) -> BOOL {
    let out = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
    out.push(h);
    true.into()
}

unsafe extern "system" fn collect_monitor(m: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
    let out = unsafe { &mut *(lparam.0 as *mut Vec<(MONITORINFO, bool)>) };
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    if unsafe { GetMonitorInfoW(m, &mut mi) }.as_bool() {
        let primary = mi.dwFlags & 1 != 0; // MONITORINFOF_PRIMARY
        out.push((mi, primary));
    }
    true.into()
}

pub struct WinBackend;

impl WinBackend {
    pub fn new() -> Self {
        Self
    }
}

impl WindowBackend for WinBackend {
    fn is_trusted(&self) -> bool {
        true
    }

    fn request_trust(&self) {}

    fn frontmost_window(&self) -> Option<WindowInfo> {
        let h = unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT) };
        if h.0.is_null() || pid_of(h) == std::process::id() || !is_app_window(h) {
            return None;
        }
        info(h)
    }

    fn list_windows(&self) -> Vec<WindowInfo> {
        let mut all: Vec<HWND> = Vec::new();
        // EnumWindows walks top to bottom in z-order.
        let _ = unsafe { EnumWindows(Some(collect_window), LPARAM(&mut all as *mut _ as isize)) };
        let own = std::process::id();
        all.into_iter()
            .filter(|h| pid_of(*h) != own && is_app_window(*h))
            .filter_map(info)
            .collect()
    }

    fn window(&self, id: WindowId) -> Option<WindowInfo> {
        let h = hwnd(id);
        if !unsafe { IsWindow(Some(h)) }.as_bool() {
            return None;
        }
        info(h)
    }

    fn set_rect(&self, id: WindowId, rect: Rect) -> Result<(), String> {
        let h = hwnd(id);
        unsafe {
            if IsZoomed(h).as_bool() || IsIconic(h).as_bool() {
                let _ = ShowWindow(h, SW_RESTORE);
            }
            // Convert the visible rect we were given into the outer rect SetWindowPos expects.
            let mut outer = RECT::default();
            GetWindowRect(h, &mut outer).map_err(|e| e.to_string())?;
            let vis = visible_rect(h).unwrap_or(outer);
            let (dl, dt) = (vis.left - outer.left, vis.top - outer.top);
            let (dr, db) = (outer.right - vis.right, outer.bottom - vis.bottom);
            let x = rect.x.round() as i32 - dl;
            let y = rect.y.round() as i32 - dt;
            let w = rect.w.round() as i32 + dl + dr;
            let hgt = rect.h.round() as i32 + dt + db;
            SetWindowPos(h, None, x, y, w, hgt, SWP_NOZORDER | SWP_NOACTIVATE).map_err(|e| e.to_string())
        }
    }

    fn focus(&self, id: WindowId) -> Result<(), String> {
        let h = hwnd(id);
        unsafe {
            if IsIconic(h).as_bool() {
                let _ = ShowWindow(h, SW_RESTORE);
            }
            if SetForegroundWindow(h).as_bool() { Ok(()) } else { Err("Windows would not bring the window forward".into()) }
        }
    }

    fn minimize(&self, id: WindowId) -> Result<(), String> {
        let _ = unsafe { ShowWindow(hwnd(id), SW_MINIMIZE) };
        Ok(())
    }

    fn monitors(&self) -> Vec<Monitor> {
        let mut found: Vec<(MONITORINFO, bool)> = Vec::new();
        let _ = unsafe { EnumDisplayMonitors(None, None, Some(collect_monitor), LPARAM(&mut found as *mut _ as isize)) };
        // Primary first, matching the macOS backend's ordering.
        found.sort_by_key(|(_, primary)| !*primary);
        found
            .into_iter()
            .enumerate()
            .map(|(i, (mi, _))| Monitor { id: i as u32, frame: to_rect(&mi.rcMonitor), work_area: to_rect(&mi.rcWork) })
            .collect()
    }

    fn launch(&self, app_id: &str) -> Result<(), String> {
        Command::new(app_id).spawn().map(|_| ()).map_err(|e| format!("could not launch {app_id}: {e}"))
    }

    fn watch(&self, id: WindowId, sink: EventSink) -> Result<Box<dyn Send>, String> {
        if !unsafe { IsWindow(Some(hwnd(id))) }.as_bool() {
            return Err("window not found".into());
        }
        ensure_hook_thread();
        let mut watchers = WATCHERS.lock().unwrap();
        let key = NEXT_KEY.fetch_add(1, Ordering::SeqCst);
        watchers.push((key, id, sink));
        Ok(Box::new(WatchGuard { key }))
    }
}

static WATCHERS: Mutex<Vec<(u64, WindowId, EventSink)>> = Mutex::new(Vec::new());
static NEXT_KEY: AtomicU64 = AtomicU64::new(1);
static HOOK_THREAD: OnceLock<()> = OnceLock::new();

struct WatchGuard {
    key: u64,
}

impl Drop for WatchGuard {
    fn drop(&mut self) {
        WATCHERS.lock().unwrap().retain(|(k, _, _)| *k != self.key);
    }
}

/// WinEvent callbacks are delivered to the thread that installed the hook, and that thread
/// must pump messages. Tauri's main thread does, but we are called from worker threads, so
/// one dedicated thread owns three process-wide hooks and routes events to watchers.
fn ensure_hook_thread() {
    HOOK_THREAD.get_or_init(|| {
        thread::spawn(|| unsafe {
            for ev in [EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_DESTROY, EVENT_SYSTEM_MINIMIZESTART] {
                SetWinEventHook(ev, ev, None, Some(win_event), 0, 0, WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS);
            }
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        });
    });
}

unsafe extern "system" fn win_event(_: HWINEVENTHOOK, event: u32, h: HWND, id_object: i32, id_child: i32, _: u32, _: u32) {
    if id_object != OBJID_WINDOW.0 || id_child != 0 {
        return;
    }
    let id = id_of(h);
    let ev = if event == EVENT_OBJECT_LOCATIONCHANGE { WindowEvent::Geometry(id) } else { WindowEvent::Gone(id) };
    let sinks: Vec<EventSink> = WATCHERS
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
