// TODO shortcuts: data driven

use crate::app;
use crate::IDEF;
use std::error::Error;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[cfg(desktop)]
pub fn init_shortcuts() -> Result<(), Box<dyn Error>> {
    let open_tray_shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyO);
    app().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, shortcut, event| {
                println!("{:?}", shortcut);
                if shortcut == &open_tray_shortcut {
                    match event.state() {
                        ShortcutState::Pressed => {
                            println!("{IDEF}: they hit the o, the poor o is hurt");
                        }
                        _ => {}
                    }
                }
            })
            .build(),
    )?;

    app().global_shortcut().register(open_tray_shortcut)?;
    Ok(())
}
