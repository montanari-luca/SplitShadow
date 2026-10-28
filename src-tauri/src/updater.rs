//! Auto-update through tauri-plugin-updater, fed by the raw JSON in `tauri.conf.json`.

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

const FIRST_CHECK_DELAY: Duration = Duration::from_secs(8);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Default)]
pub struct UpdateState {
    pub available: Mutex<Option<Update>>,
    pub error: Mutex<Option<String>>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub current: String,
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Clone)]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

pub fn status(app: &AppHandle) -> UpdateStatus {
    let st = app.state::<UpdateState>();
    let update = st.available.lock().unwrap();
    UpdateStatus {
        current: app.package_info().version.to_string(),
        available: update.is_some(),
        version: update.as_ref().map(|u| u.version.clone()),
        notes: update.as_ref().and_then(|u| u.body.clone()),
        error: st.error.lock().unwrap().clone(),
    }
}

/// Version string of a pending update, for the palette.
pub fn pending_version(app: &AppHandle) -> Option<String> {
    app.state::<UpdateState>().available.lock().unwrap().as_ref().map(|u| u.version.clone())
}

/// Human wording for the failures a bad or empty manifest produces.
fn explain(e: &tauri_plugin_updater::Error) -> String {
    let raw = e.to_string();
    if raw.contains("relative URL") || raw.contains("TargetsNotFound") || raw.contains("None of the fallback platforms") {
        format!("The update manifest is incomplete ({raw}). Each platform needs a download URL and signature.")
    } else {
        format!("Could not check for updates: {raw}")
    }
}

pub async fn check(app: &AppHandle) -> UpdateStatus {
    let result = match app.updater() {
        Ok(u) => u.check().await,
        Err(e) => Err(e),
    };
    let st = app.state::<UpdateState>();
    match result {
        Ok(update) => {
            *st.available.lock().unwrap() = update;
            *st.error.lock().unwrap() = None;
        }
        Err(e) => {
            let msg = explain(&e);
            eprintln!("updater: {msg}");
            *st.error.lock().unwrap() = Some(msg);
        }
    }
    status(app)
}

/// Download, install and relaunch. Progress goes out as `updater://progress`.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = app
        .state::<UpdateState>()
        .available
        .lock()
        .unwrap()
        .clone()
        .ok_or("No update is ready to install.")?;
    let mut downloaded = 0u64;
    let handle = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = handle.emit("updater://progress", Progress { downloaded, total });
            },
            || {},
        )
        .await
        .map_err(|e| format!("Update failed: {e}"))?;
    app.restart()
}

/// Check shortly after launch, then every few hours, without ever interrupting the user.
pub fn spawn_periodic(app: AppHandle) {
    thread::spawn(move || {
        thread::sleep(FIRST_CHECK_DELAY);
        loop {
            tauri::async_runtime::block_on(check(&app));
            thread::sleep(CHECK_EVERY);
        }
    });
}
