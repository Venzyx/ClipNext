mod base64;
mod clipboard;
mod commands;
mod db;
mod focus;
mod state;
mod tray;
mod window;

use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::db::Db;
use crate::state::{sweep_orphans, AppState, Settings, DEFAULT_SHORTCUT};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first: a second launch just reveals the window.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::show(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        window::toggle(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            // CLIPNEXT_DATA_DIR lets you run a separate profile (demos, tests).
            let data_dir = match std::env::var_os("CLIPNEXT_DATA_DIR") {
                Some(dir) => std::path::PathBuf::from(dir),
                None => app.path().app_data_dir()?,
            };
            let images_dir = data_dir.join("images");
            std::fs::create_dir_all(&images_dir)?;

            let db = Db::open(&data_dir.join("clipnext.db"))?;
            sweep_orphans(&db, &images_dir);
            let settings = Settings::load(&db);
            let language = settings.language.clone();
            app.manage(AppState::new(db, data_dir));

            tray::create(app.handle(), &language)?;

            let shortcut = settings
                .shortcut
                .parse::<Shortcut>()
                .or_else(|_| DEFAULT_SHORTCUT.parse::<Shortcut>())
                .expect("default shortcut is valid");
            if let Err(e) = app.global_shortcut().register(shortcut) {
                eprintln!("clipnext: could not register {}: {e}", settings.shortcut);
            }

            let handle = app.handle().clone();
            clipboard::start(move || clipboard::handle_change(&handle));

            // Autostart launches with --hidden so the window stays in the tray.
            if std::env::args().any(|a| a == "--hidden") {
                window::hide(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // The close button only sends the app to the tray; "Quit" in the
            // tray menu is the only way to exit.
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::Focused(false) => {
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_clips,
            commands::copy_clip,
            commands::toggle_pin,
            commands::delete_clip,
            commands::clear_clips,
            commands::clear_images,
            commands::get_settings,
            commands::set_setting,
            commands::image_usage,
            commands::hide_window,
            commands::get_thumbnail,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
