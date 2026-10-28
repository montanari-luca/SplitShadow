//! Saved workspaces: 1 to 4 windows with their geometry, persisted as JSON.

use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::wm::{Monitor, Rect, WindowBackend, WindowId, WindowInfo, monitor_for};

pub const MAX_WINDOWS: usize = 4;
const MAX_NAME: usize = 40;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsWindow {
    pub app_id: String,
    pub app: String,
    pub title_hint: String,
    pub monitor: u32,
    /// Position and size as fractions of the monitor work area.
    pub rect: Rect,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub name: String,
    pub created: u64,
    pub updated: u64,
    pub windows: Vec<WsWindow>,
    /// Indices into `windows` of two windows that were linked when saved.
    #[serde(default)]
    pub link: Option<(usize, usize)>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Store {
    pub active: Option<String>,
    pub workspaces: Vec<Workspace>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn clean_name(name: &str) -> Result<String, String> {
    let n = name.trim();
    if n.is_empty() {
        return Err("Give the workspace a name.".into());
    }
    if n.chars().count() > MAX_NAME {
        return Err(format!("Workspace names are limited to {MAX_NAME} characters."));
    }
    Ok(n.to_string())
}

impl Store {
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        // Write then rename so a crash never leaves a half-written file.
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| e.to_string())?;
        fs::rename(tmp, path).map_err(|e| e.to_string())
    }

    pub fn names(&self) -> Vec<String> {
        self.workspaces.iter().map(|w| w.name.clone()).collect()
    }

    pub fn find(&self, name: &str) -> Option<&Workspace> {
        let n = name.trim().to_lowercase();
        self.workspaces.iter().find(|w| w.name.to_lowercase() == n)
    }

    /// Create or replace by name (case-insensitive), keeping the creation date.
    pub fn upsert(&mut self, name: &str, windows: Vec<WsWindow>, link: Option<(usize, usize)>) -> Result<&Workspace, String> {
        let name = clean_name(name)?;
        let key = name.to_lowercase();
        let t = now();
        let idx = match self.workspaces.iter().position(|w| w.name.to_lowercase() == key) {
            Some(i) => {
                let created = self.workspaces[i].created;
                self.workspaces[i] = Workspace { name, created, updated: t, windows, link };
                i
            }
            None => {
                self.workspaces.push(Workspace { name, created: t, updated: t, windows, link });
                self.workspaces.len() - 1
            }
        };
        self.active = Some(self.workspaces[idx].name.clone());
        Ok(&self.workspaces[idx])
    }

    pub fn delete(&mut self, name: &str) -> Result<String, String> {
        let key = name.trim().to_lowercase();
        let i = self
            .workspaces
            .iter()
            .position(|w| w.name.to_lowercase() == key)
            .ok_or_else(|| format!("No workspace named \"{name}\"."))?;
        let removed = self.workspaces.remove(i);
        if self.active.as_deref().map(str::to_lowercase) == Some(removed.name.to_lowercase()) {
            self.active = None;
        }
        Ok(removed.name)
    }
}

fn monitor_of(monitors: &[Monitor], w: &WindowInfo) -> Option<Monitor> {
    monitor_for(monitors, &w.rect).copied()
}

/// Windows on the same display as `target`, front to back.
pub fn windows_on_target_display(backend: &dyn WindowBackend, target: WindowId) -> Result<Vec<WindowInfo>, String> {
    let monitors = backend.monitors();
    let t = backend.window(target).ok_or("The selected window is gone.")?;
    let mon = monitor_of(&monitors, &t).ok_or("No display found.")?;
    let all = backend.list_windows();
    Ok(all.into_iter().filter(|w| monitor_of(&monitors, w).map(|m| m.id) == Some(mon.id)).collect())
}

/// Snapshot windows into workspace entries. With `ids` the caller chose the windows,
/// otherwise the target plus the frontmost others on its display, up to the limit.
pub fn capture(
    backend: &dyn WindowBackend,
    target: WindowId,
    ids: Option<&[WindowId]>,
    link: Option<(WindowId, WindowId)>,
) -> Result<(Vec<WsWindow>, Option<(usize, usize)>), String> {
    let monitors = backend.monitors();
    let mut candidates = windows_on_target_display(backend, target)?;
    if !candidates.iter().any(|w| w.id == target) {
        candidates.insert(0, backend.window(target).ok_or("The selected window is gone.")?);
    }

    let chosen: Vec<WindowInfo> = match ids {
        Some(ids) => ids.iter().filter_map(|id| candidates.iter().find(|w| w.id == *id).cloned()).collect(),
        None => {
            let mut v: Vec<WindowInfo> = candidates.iter().filter(|w| w.id == target).cloned().collect();
            v.extend(candidates.iter().filter(|w| w.id != target).cloned());
            v
        }
    };
    let chosen: Vec<WindowInfo> = chosen.into_iter().take(MAX_WINDOWS).collect();
    if chosen.is_empty() {
        return Err("No windows to save.".into());
    }

    let entries = chosen
        .iter()
        .map(|w| {
            let mon = monitor_of(&monitors, w).ok_or("No display found.")?;
            Ok(WsWindow {
                app_id: w.app_id.clone(),
                app: w.app.clone(),
                title_hint: w.title.clone(),
                monitor: mon.id,
                rect: w.rect.to_fractions(&mon.work_area),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let link_idx = link.and_then(|(a, b)| {
        let ia = chosen.iter().position(|w| w.id == a)?;
        let ib = chosen.iter().position(|w| w.id == b)?;
        Some((ia, ib))
    });
    Ok((entries, link_idx))
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenReport {
    pub placed: usize,
    pub launched: usize,
    pub missing: Vec<String>,
    /// Window ids in workspace order, for restoring a link.
    pub ids: Vec<Option<WindowId>>,
}

pub struct OpenOptions {
    pub attempts: u32,
    pub interval: Duration,
}

impl Default for OpenOptions {
    fn default() -> Self {
        // About 10 seconds per app.
        Self { attempts: 40, interval: Duration::from_millis(250) }
    }
}

fn pick_window(all: &[WindowInfo], entry: &WsWindow, used: &[WindowId]) -> Option<WindowInfo> {
    let free: Vec<&WindowInfo> = all.iter().filter(|w| w.app_id == entry.app_id && !used.contains(&w.id)).collect();
    let hint = entry.title_hint.to_lowercase();
    free.iter()
        .find(|w| !hint.is_empty() && w.title.to_lowercase() == hint)
        .or_else(|| free.iter().find(|w| !hint.is_empty() && w.title.to_lowercase().contains(&hint)))
        .or_else(|| free.first())
        .map(|w| (*w).clone())
}

/// Arrange a workspace: reuse running windows, launch missing apps, place everything.
pub fn open(backend: &dyn WindowBackend, ws: &Workspace, opts: &OpenOptions) -> OpenReport {
    let monitors = backend.monitors();
    let mut report = OpenReport::default();
    let mut used: Vec<WindowId> = Vec::new();

    // Launch everything that is missing first so apps start up in parallel.
    let running = backend.list_windows();
    let mut launched_ids: Vec<&str> = Vec::new();
    for entry in &ws.windows {
        if pick_window(&running, entry, &used).is_none() && !launched_ids.contains(&entry.app_id.as_str()) {
            if backend.launch(&entry.app_id).is_ok() {
                report.launched += 1;
            }
            launched_ids.push(&entry.app_id);
        }
        if let Some(w) = pick_window(&running, entry, &used) {
            used.push(w.id);
        }
    }
    used.clear();

    for entry in &ws.windows {
        let mut found = None;
        for attempt in 0..=opts.attempts {
            found = pick_window(&backend.list_windows(), entry, &used);
            if found.is_some() || attempt == opts.attempts {
                break;
            }
            sleep(opts.interval);
        }
        match found {
            Some(w) => {
                let mon = monitors.iter().find(|m| m.id == entry.monitor).or_else(|| monitors.first());
                if let Some(mon) = mon {
                    let target = entry.rect.from_fractions(&mon.work_area);
                    if backend.set_rect(w.id, target).is_ok() {
                        report.placed += 1;
                    }
                }
                used.push(w.id);
                report.ids.push(Some(w.id));
            }
            None => {
                report.missing.push(entry.app.clone());
                report.ids.push(None);
            }
        }
    }

    // Raise back to front so the first window ends up on top.
    for id in report.ids.iter().rev().flatten() {
        let _ = backend.focus(*id);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wm::fake::FakeBackend;
    use std::sync::Arc;

    const WORK: Rect = Rect::new(0.0, 25.0, 1200.0, 800.0);

    fn backend() -> Arc<FakeBackend> {
        let b = FakeBackend::with_monitor(WORK);
        b.add_window(1, "Notes", Rect::new(0.0, 25.0, 600.0, 800.0));
        b.add_window(2, "Slack", Rect::new(600.0, 25.0, 600.0, 400.0));
        b.add_window(3, "Safari", Rect::new(600.0, 425.0, 600.0, 400.0));
        b
    }

    fn fast() -> OpenOptions {
        OpenOptions { attempts: 2, interval: Duration::from_millis(0) }
    }

    #[test]
    fn capture_puts_target_first_and_stores_fractions() {
        let b = backend();
        let (w, _) = capture(&*b, 2, None, None).unwrap();
        assert_eq!(w[0].app, "Slack");
        assert_eq!(w.len(), 3);
        assert_eq!(w[0].rect, Rect::new(0.5, 0.0, 0.5, 0.5));
    }

    #[test]
    fn capture_limits_to_four_windows() {
        let b = backend();
        b.add_window(4, "Mail", Rect::new(0.0, 25.0, 100.0, 100.0));
        b.add_window(5, "Music", Rect::new(0.0, 25.0, 100.0, 100.0));
        let (w, _) = capture(&*b, 1, None, None).unwrap();
        assert_eq!(w.len(), 4);
    }

    #[test]
    fn capture_respects_explicit_selection_and_link() {
        let b = backend();
        let (w, link) = capture(&*b, 1, Some(&[3, 1]), Some((1, 3))).unwrap();
        assert_eq!(w.iter().map(|x| x.app.as_str()).collect::<Vec<_>>(), ["Safari", "Notes"]);
        assert_eq!(link, Some((1, 0)));
    }

    #[test]
    fn open_places_running_windows() {
        let b = backend();
        let (entries, _) = capture(&*b, 1, None, None).unwrap();
        let ws = Workspace { name: "Dev".into(), created: 0, updated: 0, windows: entries, link: None };
        b.set_rect(1, Rect::new(10.0, 40.0, 300.0, 300.0)).unwrap();
        b.set_rect(2, Rect::new(10.0, 40.0, 300.0, 300.0)).unwrap();
        let r = open(&*b, &ws, &fast());
        assert_eq!(r.placed, 3);
        assert_eq!(r.launched, 0);
        assert_eq!(b.rect_of(1), Rect::new(0.0, 25.0, 600.0, 800.0));
        assert_eq!(b.rect_of(2), Rect::new(600.0, 25.0, 600.0, 400.0));
        assert_eq!(*b.frontmost.lock().unwrap(), Some(1));
    }

    #[test]
    fn open_launches_missing_apps_and_reports_ones_that_never_appear() {
        let b = backend();
        let (mut entries, _) = capture(&*b, 1, None, None).unwrap();
        entries.push(WsWindow {
            app_id: "com.test.Ghost".into(),
            app: "Ghost".into(),
            title_hint: String::new(),
            monitor: 0,
            rect: Rect::new(0.0, 0.0, 0.5, 0.5),
        });
        let ws = Workspace { name: "Dev".into(), created: 0, updated: 0, windows: entries, link: None };
        let r = open(&*b, &ws, &fast());
        assert_eq!(r.launched, 1);
        assert_eq!(r.missing, vec!["Ghost".to_string()]);
        assert_eq!(b.launched.lock().unwrap().as_slice(), ["com.test.Ghost"]);
    }

    #[test]
    fn two_windows_of_one_app_are_matched_by_title() {
        let b = FakeBackend::with_monitor(WORK);
        b.add_window(1, "Safari", Rect::new(0.0, 25.0, 100.0, 100.0));
        b.add_window(2, "Safari", Rect::new(0.0, 25.0, 100.0, 100.0));
        b.windows.lock().unwrap()[0].title = "Docs".into();
        b.windows.lock().unwrap()[1].title = "Mail".into();
        let mk = |t: &str, x: f64| WsWindow {
            app_id: "com.test.Safari".into(),
            app: "Safari".into(),
            title_hint: t.into(),
            monitor: 0,
            rect: Rect::new(x, 0.0, 0.5, 1.0),
        };
        let ws = Workspace { name: "W".into(), created: 0, updated: 0, windows: vec![mk("Mail", 0.0), mk("Docs", 0.5)], link: None };
        open(&*b, &ws, &fast());
        assert_eq!(b.rect_of(2).x, 0.0);
        assert_eq!(b.rect_of(1).x, 600.0);
    }

    #[test]
    fn missing_monitor_falls_back_to_first() {
        let b = backend();
        let e = WsWindow {
            app_id: "com.test.Notes".into(),
            app: "Notes".into(),
            title_hint: String::new(),
            monitor: 7,
            rect: Rect::new(0.0, 0.0, 0.5, 1.0),
        };
        let ws = Workspace { name: "W".into(), created: 0, updated: 0, windows: vec![e], link: None };
        let r = open(&*b, &ws, &fast());
        assert_eq!(r.placed, 1);
        assert_eq!(b.rect_of(1), Rect::new(0.0, 25.0, 600.0, 800.0));
    }

    #[test]
    fn store_upsert_find_delete() {
        let mut s = Store::default();
        s.upsert("Dev", vec![], None).unwrap();
        let created = s.find("dev").unwrap().created;
        s.upsert("DEV", vec![], None).unwrap();
        assert_eq!(s.workspaces.len(), 1);
        assert_eq!(s.find("dev").unwrap().created, created);
        assert_eq!(s.active.as_deref(), Some("DEV"));
        assert_eq!(s.delete("dev").unwrap(), "DEV");
        assert!(s.active.is_none());
        assert!(s.delete("dev").is_err());
    }

    #[test]
    fn names_are_validated() {
        assert!(clean_name("   ").is_err());
        assert!(clean_name(&"x".repeat(41)).is_err());
        assert_eq!(clean_name("  Dev  ").unwrap(), "Dev");
    }

    #[test]
    fn store_round_trips_on_disk() {
        let dir = std::env::temp_dir().join(format!("splitshadow-ws-{}", std::process::id()));
        let path = dir.join("workspaces.json");
        let mut s = Store::default();
        s.upsert("Dev", vec![], Some((0, 1))).unwrap();
        s.save(&path).unwrap();
        assert_eq!(Store::load(&path), s);
        let _ = fs::remove_dir_all(dir);
    }
}
