//! Native recovery message shown before the application can create its windows.

#[cfg(target_os = "macos")]
pub fn show_upgrade_required() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAlert, NSApplication};
    use objc2_foundation::NSString;

    if let Some(main_thread) = MainThreadMarker::new() {
        let application = NSApplication::sharedApplication(main_thread);
        application.finishLaunching();
        #[allow(deprecated)]
        application.activateIgnoringOtherApps(true);
        let alert = NSAlert::new(main_thread);
        alert.setMessageText(&NSString::from_str("请升级译羽 / Update Yiyu"));
        alert.setInformativeText(&NSString::from_str(
            "设置来自较新版本，原有数据未修改。\n\nSettings were created by a newer version; your data has not been changed.",
        ));
        alert.runModal();
    }
}

#[cfg(target_os = "windows")]
pub fn show_upgrade_required() {
    use windows::Win32::UI::WindowsAndMessaging::{MB_ICONWARNING, MB_OK, MessageBoxW};
    use windows::core::w;

    unsafe {
        let _ = MessageBoxW(
            None,
            w!(
                "设置来自较新版本，原有数据未修改。\n\nSettings were created by a newer version; your data has not been changed."
            ),
            w!("请升级译羽 / Update Yiyu"),
            MB_OK | MB_ICONWARNING,
        );
    }
}
