pub const IDEF: &str = "TDSL";
pub const FULL_APP_NAME: &str = "They Did Surgery on a Link";

mod autostart;
mod clipboard;
mod debug;
mod settings;
mod shortcuts;
mod tray;
mod surgery;
mod utils;

mod platform {
    #[cfg(target_os = "linux")]
    pub mod linux;
    #[cfg(target_os = "macos")]
    pub mod macos;
    #[cfg(target_os = "windows")]
    pub mod windows;
}

use crate::debug::{get_thread_id, start_watchdog};
#[cfg(target_os = "windows")]
use crate::platform::windows::flyout_watch;
use crate::settings::{setup_settings, Settings, SettingsState, Stage};
use crate::shortcuts::init_shortcuts;
use crate::tray::build_tray;
use crate::tray::window::{LastWanted, MAIN_WINDOW_KEY};
use std::sync::OnceLock;
use tauri::ipc::Invoke;
use tauri::{App, AppHandle, Manager, RunEvent, Runtime};
use tray::window::{Building, LastShown, PageGate};
use crate::clipboard::{ClipboardShutdown, LastClipboardWrite};

/// WARN: make sure this runs before almost everything
fn manage_state(app: &App) -> () {
    let _ = APP.set(app.handle().clone());
    app.manage(SettingsState::default());
    app.manage(LastClipboardWrite::default());
    app.manage(ClipboardShutdown::default());
    app.manage(LastShown::default());
    app.manage(LastWanted::default());
    app.manage(PageGate::default());
    app.manage(Building::default());

}

/// tauri command registration goes here
fn command_handler<R: Runtime>() -> impl Fn(Invoke<R>) -> bool {
    tauri::generate_handler![
        #[allow(unused_qualifications)]
        settings::get_settings,
        #[allow(unused_qualifications)]
        settings::set_settings
    ]
}

/// run events
fn run_event_handler(_: &AppHandle, e: RunEvent) -> () {
    match e {
        // prevents the app from closing fully if you close the window
        RunEvent::ExitRequested { code, api, .. } if code.is_none() => {
            api.prevent_exit();
        }
        _ => {}
    }
}

//

static APP: OnceLock<AppHandle> = OnceLock::new();

/// i'm a js developer give me a break
pub fn app() -> &'static AppHandle {
    APP.get()
        .unwrap_or_else(|| panic!("{IDEF}: app() called before setup"))
}

/// use instead of [`app`] where you want to prevent panics in unsafe code and etc.
pub fn try_app() -> Option<&'static AppHandle> {
    APP.get()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _ar, _c| {
            // ^ should always be the first plugin apparently
            let _ = app
                .get_webview_window(MAIN_WINDOW_KEY)
                .expect(&format!("{IDEF}: no main window"))
                .set_focus();
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .setup(|app| {
            // state
            manage_state(&app);

            // settings
            setup_settings()?;
            Settings::current().apply_stage(Stage::ASAP);

            // debug/log
            #[cfg(debug_assertions)]
            log::debug!(
                "{IDEF}: tid {} setup (this is the main thread)",
                get_thread_id()
            );
            start_watchdog();

            // setups
            autostart::setup_autostart()?;
            Settings::current().apply_stage(Stage::AtSetups); // <-- runs some setups

            // TODO now: handle macos from example (can't test)
            // TODO now: pin button to windoify with detailed settings for each disallow

            // tray/window
            // platform
            #[cfg(target_os = "linux")]
            platform::linux::disable_gpu();

            #[cfg(target_os = "windows")]
            flyout_watch::start();

            build_tray()?;

            // shortcuts
            if let Err(e) = init_shortcuts() {
                log::warn!("{IDEF}: failed to register shortcuts: {e}");
            }

            Ok(())
        })
        .invoke_handler(command_handler())
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(run_event_handler);
}
