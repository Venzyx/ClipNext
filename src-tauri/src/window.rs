use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

use crate::focus;
use crate::state::AppState;

const GAP: i32 = 8;

/// Opens the window next to the caret of the app the user is typing in and
/// remembers that app so the chosen clip can be pasted back into it.
pub fn show_near_caret(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    let ours = w.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    let fg = focus::foreground();
    let target = (fg != 0 && fg != ours).then_some(fg);
    if let Some(state) = app.try_state::<AppState>() {
        state.set_paste_target(target);
    }
    if target.is_some() {
        place_near_caret(&w);
    }
    reveal(app, &w);
}

/// Plain "open" (tray, second launch): no paste target, stays where it was.
pub fn show(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.set_paste_target(None);
    }
    if let Some(w) = app.get_webview_window("main") {
        reveal(app, &w);
    }
}

fn reveal(app: &AppHandle, w: &WebviewWindow) {
    let _ = w.unminimize();
    let _ = w.show();
    let _ = w.set_focus();
    let _ = app.emit("window-shown", ());
}

/// Puts the window just above the caret, or below it when there is no room,
/// and keeps it inside the monitor's work area.
fn place_near_caret(w: &WebviewWindow) {
    let (Some(anchor), Ok(size)) = (focus::anchor(), w.outer_size()) else {
        return;
    };
    let (width, height) = (size.width as i32, size.height as i32);
    let mut x = anchor.x - 24;
    let mut y = anchor.top - height - GAP;
    if let Some(area) = focus::work_area_at(anchor.x, anchor.top) {
        if y < area.top {
            y = anchor.bottom + GAP;
        }
        y = y.min(area.bottom - height).max(area.top);
        x = x.min(area.right - width).max(area.left);
    }
    let _ = w.set_position(PhysicalPosition::new(x, y));
}

pub fn hide(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

pub fn toggle(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    let active = w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false);
    if active {
        hide(app);
    } else {
        show_near_caret(app);
    }
}
