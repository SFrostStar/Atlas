//! GLFW (`win32_window.c`, `WM_SYSCOMMAND`) drops `SC_KEYMENU` unless its `keymenu` hint is set, so
//! Alt+Space and a bare Alt never open the system menu in vanilla. Winit forwards the message to
//! `DefWindowProc`, which opens the menu and steals the keyboard (sprint on Left Alt + jump).
//! A window subclass swallows it.

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Shell::{
    DefSubclassProc, SetCurrentProcessExplicitAppUserModelID, SetWindowSubclass,
};
#[cfg(target_pointer_width = "64")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GCLP_HICON, GCLP_HICONSM, GWLP_HINSTANCE, GetSystemMetrics, GetWindowLongPtrW, IMAGE_ICON,
    LR_SHARED, LoadImageW, SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, SetClassLongPtrW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{SC_KEYMENU, WM_SYSCOMMAND};

const SUBCLASS_ID: usize = 0x485a_4b4d;

/// Keep Explorer's game group stable across standard and locally compiled mod executables.
pub fn identify_game() {
    // SAFETY: the literal is a valid, permanently allocated UTF-16 string.
    unsafe {
        SetCurrentProcessExplicitAppUserModelID(windows_sys::core::w!("Atlas.Minecraft.Client"))
    };
}

/// Installs the game's native window icon fallback and swallows `SC_KEYMENU`.
/// Call it on the thread that owns `hwnd`.
pub fn configure_game_window(hwnd: isize) {
    let hwnd = hwnd as HWND;
    #[cfg(target_pointer_width = "64")]
    set_class_icons(hwnd);
    // SAFETY: `hwnd` is a live window of the calling thread; `procedure` is a plain function
    // that lives as long as the process, and the id and data are unused.
    unsafe {
        SetWindowSubclass(hwnd, Some(procedure), SUBCLASS_ID, 0);
    }
}

#[cfg(target_pointer_width = "64")]
fn set_class_icons(hwnd: HWND) {
    // Explorer can read class icons while the window thread is busy starting the renderer.
    // SAFETY: `hwnd` belongs to this thread. Icon 1 is owned by the game module, and LR_SHARED
    // keeps the returned handles valid for the module's lifetime.
    unsafe {
        let module = GetWindowLongPtrW(hwnd, GWLP_HINSTANCE) as _;
        for (slot, width, height) in [
            (GCLP_HICON, SM_CXICON, SM_CYICON),
            (GCLP_HICONSM, SM_CXSMICON, SM_CYSMICON),
        ] {
            let icon = LoadImageW(
                module,
                1 as *const u16,
                IMAGE_ICON,
                GetSystemMetrics(width),
                GetSystemMetrics(height),
                LR_SHARED,
            );
            if !icon.is_null() {
                SetClassLongPtrW(hwnd, slot, icon as isize);
            }
        }
    }
}

unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
    _id: usize,
    _data: usize,
) -> isize {
    if is_key_menu(message, wparam) {
        return 0;
    }
    // SAFETY: the arguments are the ones this procedure was called with.
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

/// The low four bits of `wparam` are used by the system, so they are masked off.
fn is_key_menu(message: u32, wparam: usize) -> bool {
    message == WM_SYSCOMMAND && (wparam & 0xfff0) == SC_KEYMENU as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SC_CLOSE, WM_KEYDOWN};

    #[test]
    fn only_the_key_menu_command_is_swallowed() {
        assert!(is_key_menu(WM_SYSCOMMAND, SC_KEYMENU as usize));
        assert!(is_key_menu(WM_SYSCOMMAND, SC_KEYMENU as usize | 0x2));
        assert!(!is_key_menu(WM_SYSCOMMAND, SC_CLOSE as usize));
        assert!(!is_key_menu(WM_KEYDOWN, SC_KEYMENU as usize));
    }
}
