mod capture;
mod classify;
mod watcher;

use std::borrow::Cow;
use std::thread::sleep;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::db::{Kind, NewClip};
use crate::focus;
use crate::state::{AppState, Settings};

pub use watcher::start;

const MAX_TEXT_BYTES: usize = 1024 * 1024;
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
const MAX_IMAGE_EDGE: u32 = 3840;
const THUMB_EDGE: u32 = 256;

/// Called for every clipboard change: filter, classify, store, notify the UI.
pub fn handle_change(app: &AppHandle) {
    if watcher::is_private_content() {
        return;
    }
    let state = app.state::<AppState>();
    let Some(captured) = capture::read() else {
        return;
    };
    let settings = Settings::load(&state.db.lock().unwrap());

    let clip = match captured {
        capture::Captured::Text(text) => {
            if text.len() > MAX_TEXT_BYTES {
                return;
            }
            let hash = capture::hash_text(&text);
            if state.take_suppressed(&hash) {
                return;
            }
            let kind = if classify::is_link(&text) {
                Kind::Link
            } else {
                Kind::Text
            };
            NewClip {
                kind,
                text: Some(text),
                image_path: None,
                thumb_path: None,
                image_bytes: 0,
                hash,
            }
        }
        capture::Captured::Image {
            width,
            height,
            rgba,
        } => {
            if !settings.save_images {
                return;
            }
            let hash = capture::hash_image(width, height, &rgba);
            if state.take_suppressed(&hash) {
                return;
            }
            let Some(stored) = capture::store_image(
                &state.images_dir(),
                &hash,
                width,
                height,
                rgba,
                MAX_IMAGE_BYTES,
                MAX_IMAGE_EDGE,
                THUMB_EDGE,
            ) else {
                return;
            };
            NewClip {
                kind: Kind::Image,
                text: None,
                image_path: Some(stored.image_path.to_string_lossy().into_owned()),
                thumb_path: Some(stored.thumb_path.to_string_lossy().into_owned()),
                image_bytes: stored.bytes,
                hash,
            }
        }
    };

    let result = state
        .db
        .lock()
        .unwrap()
        .insert_or_touch(&clip, &settings.limits());
    match result {
        Ok(evicted) => {
            state.remove_files(&evicted);
            let _ = app.emit("clips-changed", ());
        }
        Err(e) => eprintln!("clipnext: failed to store clip: {e}"),
    }
}

/// Puts a stored clip back on the system clipboard.
pub fn write_to_clipboard(
    state: &AppState,
    kind: &str,
    text: Option<&str>,
    image_path: Option<&str>,
) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    match kind {
        "image" => {
            let path = image_path.ok_or("image has no file")?;
            let img = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
            let (w, h) = img.dimensions();
            let bytes = img.into_raw();
            state.suppress(capture::hash_image(w as usize, h as usize, &bytes));
            cb.set_image(arboard::ImageData {
                width: w as usize,
                height: h as usize,
                bytes: Cow::Owned(bytes),
            })
            .map_err(|e| e.to_string())
        }
        _ => {
            let text = text.ok_or("clip has no text")?;
            state.suppress(capture::hash_text(text));
            cb.set_text(text.to_string()).map_err(|e| e.to_string())
        }
    }
}

/// Pastes several images into `target`, one after another. A clipboard holds a
/// single image, so each one is put on it and pasted in turn; the pause gives
/// the receiving app (chat, editor, ...) time to attach it before the next one.
/// Blocks, so run it on a worker thread.
pub fn paste_images(app: &AppHandle, target: isize, items: &[(i64, String)]) {
    let state = app.state::<AppState>();
    if !focus::activate(target) {
        return;
    }
    for (i, (id, path)) in items.iter().enumerate() {
        if write_to_clipboard(&state, "image", None, Some(path)).is_err() {
            continue;
        }
        sleep(Duration::from_millis(80));
        focus::send_paste();
        let _ = state.db.lock().unwrap().touch(*id);
        if i + 1 < items.len() {
            sleep(Duration::from_millis(450));
        }
    }
    let _ = app.emit("clips-changed", ());
}
