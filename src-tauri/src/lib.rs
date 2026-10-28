mod actions;
mod commands;
mod hotkey;
mod palette;
mod settings;
mod state;
mod tray;
mod windows;
mod workspaces;
mod wm;

use std::sync::Mutex;

use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::ShortcutState;

use crate::settings::Settings;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        palette::toggle(app);
                    }
                })
                .build(),
        )
        .invoke_handler(commands::handler())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let settings = Settings::load(&settings_path).sanitized();
            let hotkey = settings.hotkey.clone();
            let backend = wm::create_backend();
            let workspaces_path = app.path().app_data_dir()?.join("workspaces.json");
            app.manage(AppState {
                engine: wm::engine::Engine::new(backend.clone()),
                link: wm::link::LinkManager::new(backend.clone(), true),
                workspaces: Mutex::new(workspaces::Store::load(&workspaces_path)),
                workspaces_path,
                backend,
                target: Mutex::new(None),
                settings: Mutex::new(settings),
                settings_path,
                shown_at: Mutex::new(None),
            });

            let handle = app.handle().clone();
            if let Some(w) = palette::window(&handle) {
                apply_glass(&w);
                let _ = w.set_visible_on_all_workspaces(true);
                let h = handle.clone();
                w.on_window_event(move |e| {
                    if let WindowEvent::Focused(false) = e {
                        palette::on_blur(&h);
                    }
                });
            }
            if let Err(e) = hotkey::register(&handle, &hotkey) {
                eprintln!("{e}");
            }
            tray::build(&handle)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            // Stay alive in the tray when every window is closed.
            if let tauri::RunEvent::ExitRequested { api, code: None, .. } = event {
                api.prevent_exit();
            }
        });
}

fn apply_glass(w: &tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy};
        let _ = apply_vibrancy(
            w,
            NSVisualEffectMaterial::HudWindow,
            Some(NSVisualEffectState::Active),
            Some(14.0),
        );
    }
    #[cfg(target_os = "windows")]
    {
        let _ = window_vibrancy::apply_acrylic(w, Some((18, 18, 20, 110)));
    }
}
