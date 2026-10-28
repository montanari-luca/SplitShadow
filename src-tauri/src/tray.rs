use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Result};

use crate::{palette, windows};

pub fn build(app: &AppHandle) -> Result<()> {
    let open = MenuItem::with_id(app, "palette", "Open Palette", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
    let update = MenuItem::with_id(app, "update", "Check for Updates", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "About SplitShadow", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit SplitShadow", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &sep, &settings, &update, &about, &sep, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .icon_as_template(true)
        .tooltip("SplitShadow")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "palette" => palette::show(app),
            "settings" => windows::open_settings(app, None),
            "update" => windows::open_settings(app, Some("updates")),
            "about" => windows::open_settings(app, Some("about")),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                palette::toggle(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
