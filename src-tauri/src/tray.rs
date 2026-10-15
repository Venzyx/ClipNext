use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter};
use windows::Win32::Globalization::GetUserDefaultUILanguage;

use crate::window;

const LANG_TURKISH: u16 = 0x1f;

fn use_turkish(language_setting: &str) -> bool {
    match language_setting {
        "tr" => true,
        "en" => false,
        _ => {
            let lang_id = unsafe { GetUserDefaultUILanguage() };
            (lang_id & 0x3ff) == LANG_TURKISH
        }
    }
}

pub fn create(app: &AppHandle, language_setting: &str) -> tauri::Result<()> {
    let (open, settings, quit) = if use_turkish(language_setting) {
        ("Aç", "Ayarlar", "Çıkış")
    } else {
        ("Open", "Settings", "Quit")
    };
    let open_item = MenuItem::with_id(app, "open", open, true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", settings, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &settings_item, &separator, &quit_item])?;

    let mut builder = TrayIconBuilder::new()
        .tooltip("ClipNext")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => window::show(app),
            "settings" => {
                window::show(app);
                let _ = app.emit("open-settings", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::toggle(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
