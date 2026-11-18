#[cfg(target_os = "windows")]
use tauri::WebviewWindow;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongW, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SetWindowLongW, SetWindowPos, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

#[cfg(target_os = "windows")]
pub fn make_popup_nonactivating(window: &WebviewWindow) -> tauri::Result<()> {
    let hwnd = window.hwnd()?;
    unsafe {
        let style = GetWindowLongW(HWND(hwnd.0), GWL_EXSTYLE);
        SetWindowLongW(
            HWND(hwnd.0),
            GWL_EXSTYLE,
            style | WS_EX_NOACTIVATE.0 as i32 | WS_EX_TOOLWINDOW.0 as i32,
        );
        let _ = SetWindowPos(
            HWND(hwnd.0),
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
    Ok(())
}
