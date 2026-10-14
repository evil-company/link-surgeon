use crate::{IDEF};

pub fn setup_autostart() -> tauri::Result<()> {
    use tauri_plugin_autostart::MacosLauncher;

    crate::app().plugin(tauri_plugin_autostart::init(
        MacosLauncher::LaunchAgent,
        Some(vec!["--flag1", "--flag2"]),
    ))?;

    Ok(())
}

pub fn toggle_autostart(enable: bool) -> () {
    use tauri_plugin_autostart::ManagerExt;
    let autostart_manager = crate::app().autolaunch();
    
    if enable {
        let _ = autostart_manager.enable();
        log::info!("{IDEF}: registered for autostart? {}", autostart_manager.is_enabled().unwrap());
    } else {
        let _ = autostart_manager.disable();
    }
}