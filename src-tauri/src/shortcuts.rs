// TODO shortcuts: data driven

use crate::tray::window;
use crate::utils::BoxErr;
use crate::IDEF;
use crate::{app, setting};
use tauri::{PhysicalPosition, PhysicalSize, Position, Rect, Size};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[cfg(desktop)]
pub fn init_shortcuts() -> Result<(), BoxErr> {
    let hold_clipboard = setting!(shortcut_hold_clipboard);
    let open_widget = setting!(shortcut_open_widget);
    app().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, shortcut, event| {
                let pressed = event.state() == ShortcutState::Pressed;

                if *shortcut == hold_clipboard {
                    setting!(hold_clipboard = pressed);
                } else if *shortcut == open_widget && pressed {
                    if let Ok(pos) = app().cursor_position() {
                        let cursor_pos_rect = Rect {
                            position: Position::Physical(PhysicalPosition::new(
                                pos.x as i32,
                                pos.y as i32,
                            )),
                            size: Size::Physical(PhysicalSize::new(0, 0)),
                        };

                        window::show_main_window(Option::from(cursor_pos_rect))
                            .unwrap_or_else(|e| log::error!("{IDEF}: failed to open window: {e}"));
                    }
                }
            })
            .build(),
    )?;

    app().global_shortcut().register(hold_clipboard)?;
    app().global_shortcut().register(open_widget)?;
    Ok(())
}
