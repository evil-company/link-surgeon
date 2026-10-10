use std::env::set_var;
use std::error::Error;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::webview::Color;
use tauri::{
    AppHandle, Manager, PhysicalPosition, Rect, RunEvent, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

pub const IDEF: &str = "TDSL";
const MAIN_WINDOW_KEY: &str = "link-surgeon";
const MAIN_TRAY_KEY: &str = "main";
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
                            println!("{IDEF}: they hit the o, the poor o is hurt");
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

///
/// Positions the window right above taskbar and clamps it,
/// horizontally centered to the `rect` (tray icon here)
///
fn place_above_taskbar(app: &AppHandle, window: &WebviewWindow, rect: &Rect) -> tauri::Result<()> {
    let scale = window.scale_factor()?;
    let pos = rect.position.to_physical::<f64>(scale);
    let size = rect.size.to_physical::<f64>(scale);

    let Some(monitor) = app.monitor_from_point(pos.x, pos.y)? else {
        log::warn!("{IDEF} no monitor found at pos {}, {}", { pos.x }, {
            pos.y
        });
        return Ok(());
    };
    let area = monitor.work_area(); // without taskbar
    let win = window.outer_size()?;

    let left = area.position.x;
    let right = left + area.size.width as i32;
    let bottom = area.position.y + area.size.height as i32;

    let icon_center = (pos.x + size.width / 2.0) as i32;
    let x = (icon_center - win.width as i32 / 2).clamp(left, right - win.width as i32);
    let y = bottom - win.height as i32;

    window.set_position(PhysicalPosition::new(x, y))
}

// TODO right now: separate by packages because hell

#[derive(Default)]
struct LastClosed(Mutex<Option<Instant>>);

const CLOSE_DEBOUNCE: Duration = Duration::from_millis(0);
const BLUR_CLOSE_DEBOUNCE: Duration = Duration::from_millis(200);

/// close debounce (namely so the window doesn't close from you clicking the tray icon)
fn recently_closed(app: &AppHandle) -> bool {
    app.state::<LastClosed>()
        .0
        .lock()
        .unwrap()
        .is_some_and(|t| t.elapsed() < CLOSE_DEBOUNCE)
}

fn open_main_window(app: &AppHandle, anchor: Option<Rect>) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window(MAIN_WINDOW_KEY) {
        w.show()?;
        w.set_focus()?;
        return Ok(());
    };

    let mut builder = WebviewWindowBuilder::new(app, MAIN_WINDOW_KEY, WebviewUrl::default())
        .title("Settings")
        .inner_size(400., 600.)
        .min_inner_size(400., 400.)
        .background_color(Color(24, 24, 27, 255))
        .skip_taskbar(true)
        .resizable(false); // TODO soon: make flash background color match ui

    // TODO now: don't start until UI done

    #[cfg(target_os = "windows")]
    {
        // disables gpu acceleration
        // other args are required as it overrides wry defaults
        const WINDOW_ARGS: &str =
            "--disable-gpu --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";

        builder = builder
            .transparent(true)
            .decorations(false)
            .additional_browser_args(WINDOW_ARGS);
    }

    let window = builder.visible(false).build()?;
    if let Some(rect) = &anchor {
        place_above_taskbar(app, &window, rect)
            .unwrap_or_else(|e| log::warn!("{IDEF} failed to position window {e}"));
    };
    let opened = Instant::now();

    window.show()?;
    window.set_focus()?;

    #[cfg(target_os = "windows")]
    {
        let _ = window.set_shadow(true);
        let _ = window_vibrancy::apply_blur(&window, Some((0, 0, 0, 0)));
    }

    // closes the window if unfocused
    let w = window.clone();
    let a = app.clone();
    window.on_window_event(move |e| {
        if let WindowEvent::Focused(false) = e {
            if opened.elapsed() < BLUR_CLOSE_DEBOUNCE {
                let _ = w.set_focus();
                return;
            };
            *a.state::<LastClosed>().0.lock().unwrap() = Some(Instant::now());
            let _ = w.close();
        }
    });

    #[cfg(debug_assertions)]
    {
        // window.open_devtools();
    }

    Ok(())
}

// Disabling GPU acceleration saves ~40MB of RAM
// which is less than I expected,
// not that it matters since it has no impact
// unless you're looking through settings but still

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // TODO now: test on linux if gpu acceleration is off
    // supposedly disables gpu acceleration on Linux
    set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .setup(|app| {
            // TODO right now: handle tray events and setup
            // TODO right now: handle window events
            // TODO right now: handle wevbview spawn (maybe positioner plugin?)
            // TODO now: handle macos from example (can't test)
            // TODO now: pin button to windoify with detailed settings for each disallow
            // TODO soon: clean dependencies

            let app_handle = app.handle();

            // tray init
            let open_i = MenuItem::with_id(
                app,
                "open",
                "Open They Did Surgery on a Link",
                true,
                None::<&str>,
            )?;
            let quit_i = MenuItem::with_id(
                app,
                "quit",
                "Quit Open They Did Surgery on a Link",
                true,
                None::<&str>,
            )?;
            let menu = Menu::with_items(app, &[&open_i, &quit_i])?;

            app.manage(LastClosed::default());

            TrayIconBuilder::with_id(MAIN_TRAY_KEY)
                .menu(&menu)
                .icon(APP_ICON)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    match {
                        tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                        event
                    } {
                        TrayIconEvent::Click {
                            rect,
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } if !recently_closed(tray.app_handle()) => {
                            open_main_window(tray.app_handle(), Some(rect)).unwrap_or_else(|e| {
                                log::error!("{IDEF}: failed to open window: {e}")
                            })
                        }
                        _ => {}
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        let anchor = app
                            .tray_by_id(MAIN_TRAY_KEY)
                            .and_then(|t| t.rect().ok().flatten());
                        open_main_window(app, anchor)
                            .unwrap_or_else(|e| log::error!("{IDEF}: failed to open window: {e}"));
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => panic!("{IDEF}: there's an unhandled menu option!"),
                })
                .build(app)?;

            // shortcuts
            init_shortcuts(&app_handle)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, e| match e {
            // prevents the app from closing fully if you close the window
            RunEvent::ExitRequested { code, api, .. } if code.is_none() => {
                api.prevent_exit();
            }
            _ => {}
        });
}
