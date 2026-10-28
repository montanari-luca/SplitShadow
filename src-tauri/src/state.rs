use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::settings::Settings;
use crate::wm::{WindowBackend, WindowInfo};

pub struct AppState {
    pub backend: Arc<dyn WindowBackend>,
    /// Window that was frontmost when the palette opened.
    pub target: Mutex<Option<WindowInfo>>,
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
    pub shown_at: Mutex<Option<Instant>>,
}
