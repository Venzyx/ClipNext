use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::clipboard;
use crate::db::Clip;
use crate::state::{write_setting, AppState, Settings};
use crate::window;

const PREVIEW_CHARS: usize = 400;

fn err<E: ToString>(e: E) -> String {
    e.to_string()
}

#[tauri::command]
pub fn list_clips(state: State<'_, AppState>, query: Option<String>) -> Result<Vec<Clip>, String> {
    state
        .db
        .lock()
        .unwrap()
        .list(query.as_deref(), PREVIEW_CHARS)
        .map_err(err)
}

#[tauri::command]
pub fn copy_clip(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let clip = state
        .db
        .lock()
        .unwrap()
        .get(id)
        .map_err(err)?
        .ok_or("clip not found")?;
    let full_text = state.db.lock().unwrap().full_text(id).map_err(err)?;
    clipboard::write_to_clipboard(
        &state,
        &clip.kind,
        full_text.as_deref(),
        clip.image_path.as_deref(),
    )?;
    state.db.lock().unwrap().touch(id).map_err(err)?;
    let auto_paste = Settings::load(&state.db.lock().unwrap()).auto_paste;
    window::hide(&app);
    if let Some(target) = state.take_paste_target() {
        if auto_paste {
            crate::focus::paste_into(target);
        }
    }
    let _ = app.emit("clips-changed", ());
    Ok(())
}

#[tauri::command]
pub fn toggle_pin(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    pinned: bool,
) -> Result<(), String> {
    state
        .db
        .lock()
        .unwrap()
        .set_pinned(id, pinned)
        .map_err(err)?;
    let _ = app.emit("clips-changed", ());
    Ok(())
}

#[tauri::command]
pub fn delete_clip(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let files = state.db.lock().unwrap().delete(id).map_err(err)?;
    state.remove_files(&files);
    let _ = app.emit("clips-changed", ());
    Ok(())
}

#[tauri::command]
pub fn clear_clips(
    app: AppHandle,
    state: State<'_, AppState>,
    include_pinned: bool,
) -> Result<(), String> {
    let files = state
        .db
        .lock()
        .unwrap()
        .clear(include_pinned)
        .map_err(err)?;
    state.remove_files(&files);
    let _ = app.emit("clips-changed", ());
    Ok(())
}

#[tauri::command]
pub fn clear_images(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let files = state.db.lock().unwrap().clear_images().map_err(err)?;
    state.remove_files(&files);
    let _ = app.emit("clips-changed", ());
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    Settings::load(&state.db.lock().unwrap())
}

#[tauri::command]
pub fn set_setting(
    app: AppHandle,
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<Settings, String> {
    if key == "autostart" {
        let launcher = app.autolaunch();
        let result = if value == "true" {
            launcher.enable()
        } else {
            launcher.disable()
        };
        result.map_err(err)?;
    }
    if key == "shortcut" {
        let new: Shortcut = value
            .parse()
            .map_err(|_| "Invalid shortcut".to_string())?;
        if new.mods.is_empty() {
            return Err("Use at least one modifier key (Ctrl, Alt, Shift or Win)".to_string());
        }
        let old = Settings::load(&state.db.lock().unwrap())
            .shortcut
            .parse::<Shortcut>()
            .ok();
        if old != Some(new) {
            let shortcuts = app.global_shortcut();
            shortcuts
                .register(new)
                .map_err(|_| "This shortcut is already in use by another app".to_string())?;
            if let Some(old) = old {
                let _ = shortcuts.unregister(old);
            }
        }
    }
    let (settings, evicted) = {
        let mut db = state.db.lock().unwrap();
        write_setting(&mut db, &key, &value)?;
        let settings = Settings::load(&db);
        let evicted = db.enforce_limits(&settings.limits()).map_err(err)?;
        (settings, evicted)
    };
    state.remove_files(&evicted);
    let _ = app.emit("clips-changed", ());
    Ok(settings)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUsage {
    pub bytes: i64,
    pub count: i64,
}

#[tauri::command]
pub fn image_usage(state: State<'_, AppState>) -> Result<ImageUsage, String> {
    let (bytes, count) = state.db.lock().unwrap().image_usage().map_err(err)?;
    Ok(ImageUsage { bytes, count })
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    window::hide(&app);
}

/// Thumbnail as a `data:` URL so the webview needs no file-system access.
#[tauri::command]
pub fn get_thumbnail(state: State<'_, AppState>, id: i64) -> Result<String, String> {
    let clip = state
        .db
        .lock()
        .unwrap()
        .get(id)
        .map_err(err)?
        .ok_or("clip not found")?;
    let path = clip.thumb_path.ok_or("clip has no thumbnail")?;
    let bytes = std::fs::read(path).map_err(err)?;
    Ok(format!("data:image/png;base64,{}", crate::base64::encode(&bytes)))
}

/// Pastes the selected images one after another into the app that had the
/// focus when the panel opened. Without a paste target (or with auto-paste
/// off) only the last selected image is copied to the clipboard.
#[tauri::command]
pub fn paste_clips(app: AppHandle, state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), String> {
    let mut images: Vec<(i64, String)> = Vec::new();
    {
        let db = state.db.lock().unwrap();
        for id in ids {
            if let Some(clip) = db.get(id).map_err(err)? {
                if clip.kind == "image" {
                    if let Some(path) = clip.image_path {
                        images.push((id, path));
                    }
                }
            }
        }
    }
    if images.is_empty() {
        return Err("no images selected".to_string());
    }
    let auto_paste = Settings::load(&state.db.lock().unwrap()).auto_paste;
    let target = state.take_paste_target();
    window::hide(&app);

    match (auto_paste, target) {
        (true, Some(target)) => {
            std::thread::spawn(move || clipboard::paste_images(&app, target, &images));
        }
        _ => {
            let (id, path) = images.last().expect("checked non-empty");
            clipboard::write_to_clipboard(&state, "image", None, Some(path))?;
            state.db.lock().unwrap().touch(*id).map_err(err)?;
            let _ = app.emit("clips-changed", ());
        }
    }
    Ok(())
}
