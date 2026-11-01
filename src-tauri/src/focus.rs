//! Win32 helpers that make the window behave like the Windows clipboard
//! panel: remember where the user was typing, open next to the caret, and
//! paste back into that window.

use std::ffi::c_void;
use std::mem::size_of;
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_CONTROL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, IsWindow,
    SetForegroundWindow, GUITHREADINFO,
};

const VK_V: VIRTUAL_KEY = VIRTUAL_KEY(0x56);

/// A point just above/below where the user is typing (screen pixels).
#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    pub x: i32,
    pub top: i32,
    pub bottom: i32,
}

fn hwnd_from(value: isize) -> HWND {
    HWND(value as *mut c_void)
}

/// The window that currently has the keyboard focus, as an opaque handle.
pub fn foreground() -> isize {
    unsafe { GetForegroundWindow() }.0 as isize
}

/// Caret position of the foreground window, falling back to the mouse
/// cursor for apps that do not expose a classic caret (browsers, Electron).
pub fn anchor() -> Option<Anchor> {
    unsafe {
        let fg = GetForegroundWindow();
        if !fg.0.is_null() {
            let thread = GetWindowThreadProcessId(fg, None);
            let mut info = GUITHREADINFO {
                cbSize: size_of::<GUITHREADINFO>() as u32,
                ..Default::default()
            };
            if GetGUIThreadInfo(thread, &mut info).is_ok() && !info.hwndCaret.0.is_null() {
                let r = info.rcCaret;
                if r.right > r.left || r.bottom > r.top {
                    let mut top = POINT { x: r.left, y: r.top };
                    let mut bottom = POINT { x: r.left, y: r.bottom };
                    if ClientToScreen(info.hwndCaret, &mut top).as_bool()
                        && ClientToScreen(info.hwndCaret, &mut bottom).as_bool()
                    {
                        return Some(Anchor {
                            x: top.x,
                            top: top.y,
                            bottom: bottom.y,
                        });
                    }
                }
            }
        }
        let mut p = POINT::default();
        GetCursorPos(&mut p).ok()?;
        Some(Anchor {
            x: p.x,
            top: p.y,
            bottom: p.y + 20,
        })
    }
}

/// Usable screen area (excluding the taskbar) of the monitor at a point.
pub fn work_area_at(x: i32, y: i32) -> Option<RECT> {
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(monitor, &mut info)
            .as_bool()
            .then_some(info.rcWork)
    }
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Brings `target` back to the foreground. Blocks briefly, so call it from a
/// worker thread.
pub fn activate(target: isize) -> bool {
    unsafe {
        let hwnd = hwnd_from(target);
        if !IsWindow(Some(hwnd)).as_bool() {
            return false;
        }
        // Let our window finish hiding so focus can return cleanly.
        thread::sleep(Duration::from_millis(60));
        let _ = SetForegroundWindow(hwnd);
        thread::sleep(Duration::from_millis(80));
        true
    }
}

/// Sends Ctrl+V to whichever window currently has the focus.
pub fn send_paste() {
    let inputs = [
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ];
    unsafe {
        SendInput(&inputs, size_of::<INPUT>() as i32);
    }
}

/// Brings `target` back to the foreground and pastes into it.
/// Runs on its own thread so the UI command returns immediately.
pub fn paste_into(target: isize) {
    thread::spawn(move || {
        if activate(target) {
            send_paste();
        }
    });
}
