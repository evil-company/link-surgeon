use std::error::Error;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

const MAIN_WINDOW_KEY: &str = "link-surgeon";
const APP_ICON: Image<'_> = tauri::include_image!("./icons/128x128.png");

#[cfg(desktop)]
fn init_shortcuts(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let open_tray_shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyO);
    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, shortcut, event| {
                println!("{:?}", shortcut);
                if shortcut == &open_tray_shortcut {
                    match event.state() {
                        ShortcutState::Pressed => {
                            println!("TDSL: they hit the o, the poor o is hurt");
                        }
                        _ => {}
                    }
                }
            })
            .build(),
    )?;

    app.global_shortcut().register(open_tray_shortcut)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // TODO right now: handle tray events and setup
            // TODO right now: handle window events
            // TODO right now: handle wevbview spawn (maybe positioner plugin?)
            // TODO now: handle macos from example (can't test)

            // tray init
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_i])?;

            #[allow(unused)]
            let tray = TrayIconBuilder::new()
                .menu(&menu)
                .icon(APP_ICON)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    _ => panic!("tdsl: there's an unhandled menu option!"),
                })
                .build(app)?;

            // window
            let mut window_builder =
                WebviewWindowBuilder::new(app, MAIN_WINDOW_KEY, WebviewUrl::default())
                    .title("Settings")
                    .inner_size(1000., 800.)
                    .min_inner_size(600., 400.);

            #[cfg(target_os = "windows")]
            {
                window_builder = window_builder.transparent(true);
                window_builder = window_builder.decorations(false);
            }

            let window = window_builder.build().unwrap();

            // need? (from example)
            #[cfg(target_os = "windows")]
            {
                let _ = window.set_shadow(true);
                let _ = window_vibrancy::apply_blur(&window, Some((0, 0, 0, 0)));
            }

            // need? (from example)
            #[cfg(debug_assertions)]
            window.open_devtools();

            let app_handle = app.handle();

            // shortcuts
            init_shortcuts(&app_handle)?;

            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                // TODO now: figure if need close dialog (no?), from example
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![greet])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, e| match e {
            // this prevents the app from closing fully if you close the window
            RunEvent::ExitRequested { code, api, .. } => {
                if code.is_none() {
                    api.prevent_exit(); }
            }
            _ => {}
        });
}
