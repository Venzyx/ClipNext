//! Event-driven clipboard listener (no polling): a hidden message-only window
//! registered with `AddClipboardFormatListener` receives WM_CLIPBOARDUPDATE.

use std::sync::OnceLock;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HGLOBAL, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, GetClipboardData, IsClipboardFormatAvailable,
    OpenClipboard, RegisterClipboardFormatW,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
    TranslateMessage, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLIPBOARDUPDATE,
    WNDCLASSW,
};

static CALLBACK: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_CLIPBOARDUPDATE {
        if let Some(cb) = CALLBACK.get() {
            cb();
        }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Starts the listener on its own thread. `on_change` runs on that thread
/// for every clipboard change.
pub fn start(on_change: impl Fn() + Send + Sync + 'static) {
    let _ = CALLBACK.set(Box::new(on_change));
    std::thread::spawn(|| unsafe {
        let Ok(module) = GetModuleHandleW(None) else {
            return;
        };
        let hinst: HINSTANCE = module.into();
        let class = w!("ClipNextClipboardListener");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinst,
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassW(&wc);
        let Ok(hwnd) = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            w!("ClipNext"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(hinst),
            None,
        ) else {
            return;
        };
        if AddClipboardFormatListener(hwnd).is_err() {
            return;
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

unsafe fn format_present(name: PCWSTR) -> bool {
    let id = unsafe { RegisterClipboardFormatW(name) };
    id != 0 && unsafe { IsClipboardFormatAvailable(id) }.is_ok()
}

/// True if the format is present and its DWORD payload is 0.
unsafe fn dword_is_zero(name: PCWSTR) -> bool {
    let id = unsafe { RegisterClipboardFormatW(name) };
    if id == 0 || unsafe { IsClipboardFormatAvailable(id) }.is_err() {
        return false;
    }
    if unsafe { OpenClipboard(None) }.is_err() {
        return false;
    }
    let mut zero = false;
    if let Ok(handle) = unsafe { GetClipboardData(id) } {
        let global = HGLOBAL(handle.0);
        let ptr = unsafe { GlobalLock(global) } as *const u32;
        if !ptr.is_null() {
            zero = unsafe { *ptr } == 0;
            let _ = unsafe { GlobalUnlock(global) };
        }
    }
    let _ = unsafe { CloseClipboard() };
    zero
}

/// Password managers mark sensitive copies with these clipboard formats.
/// Such content must never be stored.
pub fn is_private_content() -> bool {
    unsafe {
        format_present(w!("ExcludeClipboardContentFromMonitorProcessing"))
            || format_present(w!("Clipboard Viewer Ignore"))
            || dword_is_zero(w!("CanIncludeInClipboardHistory"))
    }
}
