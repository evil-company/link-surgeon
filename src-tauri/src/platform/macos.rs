// TODO macos handle

use crate::app;

/// TODO macos: can't say what this does for sure, needs a vm test
/// TODO macos: test autostart
#[cfg(target_os = "macos")]
fn set_macos_activation_policy() -> () {
    app()
        .set_activation_policy(tauri::ActivationPolicy::Accessory)
        .unwrap();
}
