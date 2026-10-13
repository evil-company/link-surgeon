pub mod window;

pub const MAIN_TRAY_KEY: &str = "main";
pub const APP_ICON: Image<'_> = tauri::include_image!("./icons/128x128.png");

use crate::app;
use crate::FULL_APP_NAME;
use crate::IDEF;
use const_format::formatcp;
use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

// events

fn tray_event_handler(_: &TrayIcon, e: TrayIconEvent) -> () {
    match e {
        TrayIconEvent::Enter { .. } => {
            if let Err(e) = window::ensure_main_window() {
                log::warn!("{IDEF}: failed to pre-build window: {e}")
            }
        }
        TrayIconEvent::Click {
            rect,
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } => window::show_main_window(Some(rect))
            .unwrap_or_else(|e| log::error!("{IDEF}: failed to open window: {e}")),
        _ => {}
    }
}

// menu

// could add other args but useless right now
struct TrayEntry {
    id: &'static str,
    text: &'static str,
    fun: fn(&AppHandle, MenuEvent),
}

const TRAY_ENTRIES: &[TrayEntry] = &[
    TrayEntry {
        id: "open",
        text: formatcp!("Open {}", FULL_APP_NAME),
        fun: |app, _| {
            let anchor = app
                .tray_by_id(MAIN_TRAY_KEY)
                .and_then(|t| t.rect().ok().flatten());
            window::show_main_window(anchor)
                .unwrap_or_else(|e| log::error!("{IDEF}: failed to open window: {e}"));
        },
    },
    TrayEntry {
        id: "quit",
        text: formatcp!("Quit {}", FULL_APP_NAME),
        fun: |app, _| {
            app.exit(0);
        },
    },
];

pub fn build_tray() -> tauri::Result<()> {
    let menu = Menu::new(app())?;
    for e in TRAY_ENTRIES {
        menu.append(&MenuItem::with_id(app(), e.id, e.text, true, None::<&str>)?)?;
    }

    TrayIconBuilder::with_id(MAIN_TRAY_KEY)
        .menu(&menu)
        .icon(APP_ICON)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(tray_event_handler)
        .on_menu_event(|app, event| {
            if let Some(e) = TRAY_ENTRIES.iter().find(|e| e.id == event.id().0) {
                (e.fun)(app, event);
            }
        })
        .build(app())?;

    Ok(())
}
