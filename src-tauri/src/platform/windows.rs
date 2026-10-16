use crate::setting;
use tauri::{AppHandle, WebviewWindow, WebviewWindowBuilder, Wry};

/// disables the window spawning transition
///
/// decided to keep them
#[cfg(target_os = "windows")]
fn _disable_windows_transitions(window: &WebviewWindow) {
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED};

    let Ok(hwnd) = window.hwnd() else { return };
    let disable: i32 = 1;
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &disable as *const i32 as *const _,
            size_of::<i32>() as u32,
        );
    }
}

/// TODO linux: linux version of flyout watch if can
/// TODO windows 10: check if flyout watch works on win10
///
/// On Windows preloads on tray opening to make it feel instant.<br>
/// Is this a bit disrespectful towards other tray apps? So may it be, we're mean after all.
///
#[cfg(target_os = "windows")]
pub mod flyout_watch {
    use crate::tray::window::ensure_main_window;
    use crate::{setting, try_app, IDEF};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT,
    };

    // errors inside here crash the whole app
    unsafe extern "system" fn on_event(
        _hook: HWINEVENTHOOK,
        _event: u32,
        hwnd: HWND,
        _id_object: i32,
        _id_child: i32,
        _thread: u32,
        _time: u32,
    ) {
        let mut buf = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut buf) as usize;
        let class = String::from_utf16_lossy(&buf[..len]);

        if setting!(debug_log_foreground_class) {
            #[cfg(debug_assertions)]
            log::debug!("{IDEF} foreground class: {class}");
        }

        if class == "NotifyIconOverflowWindow" || class == "TopLevelWindowForOverflowXamlIsland" {
            if try_app().is_some() {
                let _ = ensure_main_window();
            }
        }
    }

    /// run on the main thread
    pub fn start() {
        unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(on_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
        }
    }
}

#[cfg(target_os = "windows")]
pub fn set_windows_window_builder_settings(
    mut builder: WebviewWindowBuilder<Wry, AppHandle>,
) -> WebviewWindowBuilder<Wry, AppHandle> {
    if !setting!(gpu) {
        // other args are required as it overrides wry defaults

        const DISABLE_GPU_WINDOW_ARGS: &str =
            "--disable-gpu --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";

        builder = builder.additional_browser_args(DISABLE_GPU_WINDOW_ARGS);
    }

    builder
}

#[cfg(target_os = "windows")]
pub fn _set_windows_window_settings(window: &WebviewWindow) {
    //
}
