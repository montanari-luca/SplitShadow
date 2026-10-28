use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const SETTINGS: &str = "settings";

pub fn open_settings(app: &AppHandle, tab: Option<&str>) {
    if let Some(w) = app.get_webview_window(SETTINGS) {
        let _ = w.show();
        let _ = w.set_focus();
        if let Some(t) = tab {
            let _ = app.emit_to(SETTINGS, "settings://tab", t);
        }
        return;
    }
    let url = match tab {
        Some(t) => format!("index.html?tab={t}"),
        None => "index.html".to_string(),
    };
    let built = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App(url.into()))
        .title("SplitShadow")
        .inner_size(780.0, 560.0)
        .min_inner_size(640.0, 460.0)
        .center()
        .build();
    if let Ok(w) = built {
        let _ = w.set_focus();
    }
}
