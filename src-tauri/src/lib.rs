use std::env::set_var;
use std::error::Error;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::webview::{Color, PageLoadEvent};
use tauri::{
    App, AppHandle, Manager, PhysicalPosition, Rect, RunEvent, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// TODO now: settings json ; or tauri stor
// TODO now: settings rust-to-js sync

// SETTINGS

// Originally I disabled gpu by default to save on RAM,
// but it's only ~40MB, and it literally doesn't matter since it's
// folded and uses 5MB RAM unless open anyway.
// Keeping it on makes the webview's initial draw basically instant

#[derive(Default)]
struct SettingsState(Mutex<Settings>);

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
struct Settings {
    pin: bool, // TODO pin: pin setting
    gpu: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            pin: false,
            gpu: true,
        }
    }
}
impl Settings {
    fn with<T>(f: impl FnOnce(&Settings) -> T) -> T {
        f(&app().state::<SettingsState>().0.lock().unwrap())
    }

    fn current() -> Settings {
        Self::with(|s| s.clone())
    }
}

macro_rules! setting {
    ($field:ident) => {
        Settings::with(|s| s.$field.clone())
    };
}

//

pub const IDEF: &str = "TDSL";
const MAIN_WINDOW_KEY: &str = "link-surgeon";
const MAIN_TRAY_KEY: &str = "main";
const APP_ICON: Image<'_> = tauri::include_image!("./icons/128x128.png");

#[cfg(desktop)]
fn init_shortcuts() -> Result<(), Box<dyn Error>> {
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

const TASKBAR_MARGIN: i32 = 10; // px

///
/// Positions the window right above taskbar and clamps it,
/// horizontally centered to the `rect` (tray icon here)
///
fn place_above_taskbar(
    window: &WebviewWindow,
    rect: &Rect,
    bottom_margin: i32,
) -> tauri::Result<()> {
    let scale = window.scale_factor()?;
    let pos = rect.position.to_physical::<f64>(scale);
    let size = rect.size.to_physical::<f64>(scale);

    let Some(monitor) = app().monitor_from_point(pos.x, pos.y)? else {
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
    let y = bottom - win.height as i32 - bottom_margin;

    window.set_position(PhysicalPosition::new(x, y))
}

// TODO right now: separate by packages because hell

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
struct Building(AtomicBool);

/// TODO linux: linux version of flyout watch
/// TODO windows 10: check if flyout watch works on win10
///
/// On Windows preloads on tray opening to make it feel instant.
/// Is this a bit disrespectful towards other tray apps? So may it be, we're mean after all.
///
#[cfg(target_os = "windows")]
mod flyout_watch {
    use super::*;

    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT,
    };

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

        #[cfg(debug_assertions)]
        log::debug!("{IDEF} foreground class: {class}");

        if class == "NotifyIconOverflowWindow" || class == "TopLevelWindowForOverflowXamlIsland" {
            if try_app().is_some() { let _ = ensure_main_window();}
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
fn disable_windows_transitions(window: &WebviewWindow) {
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

#[derive(Default)]
struct LastClosed(Mutex<Option<Instant>>);

const CLOSE_DEBOUNCE: Duration = Duration::from_millis(200); // TODO test manual: need?
const BLUR_CLOSE_DEBOUNCE: Duration = Duration::from_millis(200);

/// close debounce (namely so the window doesn't close from you clicking the tray icon)
fn recently_closed() -> bool {
    app().state::<LastClosed>()
        .0
        .lock()
        .unwrap()
        .is_some_and(|t| t.elapsed() < CLOSE_DEBOUNCE)
}

#[derive(Default)]
struct LastShown(Mutex<Option<Instant>>);
const PREOPEN_TIMEOUT: Duration = Duration::from_secs(20);
fn recently_shown() -> bool {
    app().state::<LastShown>()
        .0
        .lock()
        .unwrap()
        .is_some_and(|t| t.elapsed() < BLUR_CLOSE_DEBOUNCE)
}


#[derive(Default)]
struct GateInner {
    loaded: bool,
    pending: Option<Option<Rect>>,
}

#[derive(Default)]
struct PageGate(Mutex<GateInner>);

fn reveal(anchor: Option<Rect>) -> tauri::Result<()> {
    let Some(window) = app().get_webview_window(MAIN_WINDOW_KEY) else {
        return Ok(());
    };

    if let Some(rect) = &anchor {
        place_above_taskbar(&window, rect, TASKBAR_MARGIN)
            .unwrap_or_else(|e| log::warn!("{IDEF} failed to position window {e}"));
    };

    *app().state::<LastShown>().0.lock().unwrap() = Some(Instant::now());
    window.show()?;
    window.set_focus()?;
    Ok(())
}

fn build_main_window() -> tauri::Result<()> {
    let mut builder = WebviewWindowBuilder::new(app(), MAIN_WINDOW_KEY, WebviewUrl::default())
        .title("Settings")
        .inner_size(400., 600.)
        .min_inner_size(400., 400.)
        .background_color(Color(47, 47, 47, 255)) // TODO soon: make flash background color match ui
        .skip_taskbar(true)
        .always_on_top(true)
        .resizable(false)
        .visible(false)
        .on_page_load(|_, payload| {
            if let PageLoadEvent::Finished = payload.event() {
                let pending = {
                    let gate = app().state::<PageGate>();
                    let mut g = gate.0.lock().unwrap();
                    g.loaded = true;
                    g.pending.take()
                };
                if let Some(anchor) = pending {
                    let _ = reveal(anchor);
                }
            }
        });

    #[cfg(target_os = "windows")]
    {
        builder = builder.transparent(true).decorations(false);

        if !setting!(gpu) {
            // other args are required as it overrides wry defaults

            const DISABLE_GPU_WINDOW_ARGS: &str =
                "--disable-gpu --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";

            builder = builder.additional_browser_args(DISABLE_GPU_WINDOW_ARGS);
        }
    }

    let window = builder.build()?;

    #[cfg(target_os = "windows")]
    {
        // TODO pin setting: make depend on widget setting? no need if always on on decorate window? would be decorated?
        // TODO maybe: maybe close tray immediately instead of window's delay as a minor aesthetic nit
        disable_windows_transitions(&window);
        let _ = window.set_shadow(false);
        let _ = window_vibrancy::apply_blur(&window, Some((0, 0, 0, 0)));
    }

    // closes the window if unfocused
    let w = window.clone();
    window.on_window_event(move |e| {
        if let WindowEvent::Focused(false) = e {
            if !w.is_visible().unwrap_or(false) {
                return;
            }
            if recently_shown() {
                let _ = w.set_focus();
                return;
            }
            *app().state::<LastClosed>().0.lock().unwrap() = Some(Instant::now());
            let _ = w.close();
        }
    });

    // prebuilds on hover, closes if timed out
    std::thread::spawn(move || {
        std::thread::sleep(PREOPEN_TIMEOUT);
        if let Some(w) = app().get_webview_window(MAIN_WINDOW_KEY) {
            if !w.is_visible().unwrap_or(true) {
                let _ = w.close();
            }
        }
    });

    Ok(())
}

fn ensure_main_window() -> tauri::Result<()> {
    if app().get_webview_window(MAIN_WINDOW_KEY).is_some() {
        return Ok(());
    }

    if app().state::<Building>().0.swap(true, Ordering::SeqCst) {
        log::debug!("{IDEF}: {} ensure: skipped, build already running", tid());
        return Ok(());
    }

    {
        let gate = app().state::<PageGate>();
        let mut g = gate.0.lock().unwrap();
        g.loaded = false;
        g.pending = None;
    }

    let result = build_main_window();
    app().state::<Building>().0.store(false, Ordering::SeqCst);

    let ready = {
        let gate = app().state::<PageGate>();
        let mut g = gate.0.lock().unwrap();
        if g.loaded {
            g.pending.take()
        } else {
            None
        }
    };
    if let Some(anchor) = ready {
        let _ = reveal(anchor);
    }

    result
}

fn show_main_window(anchor: Option<Rect>) -> tauri::Result<()> {
    ensure_main_window()?;
    let gate = app().state::<PageGate>();
    let mut g = gate.0.lock().unwrap();
    if g.loaded {
        drop(g);
        reveal(anchor)
    } else {
        g.pending = Some(anchor);
        Ok(())
    }
}

fn manage_state(app: &App) -> () {
    let _ = APP.set(app.handle().clone());
    app.manage(Settings::default());
    app.manage(SettingsState::default());
    app.manage(LastClosed::default());
    app.manage(LastShown::default());
    app.manage(PageGate::default());
    app.manage(Building::default());
}

/// i'm a js developer give me a break
static APP: OnceLock<AppHandle> = OnceLock::new();

fn app() -> &'static AppHandle {
    APP.get()
        .unwrap_or_else(|| panic!("{IDEF}: app() called before setup"))
}

/// use where you want to prevent panics in unsafe code
fn try_app() -> Option<&'static AppHandle> {
    APP.get()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .setup(|app| {
            manage_state(&app);

            //

            // TODO right now: handle tray events and setup
            // TODO right now: handle window events
            // TODO right now: handle wevbview spawn (maybe positioner plugin?)
            // TODO now: handle macos from example (can't test)
            // TODO now: pin button to windoify with detailed settings for each disallow
            // TODO soon: clean dependencies

            #[cfg(debug_assertions)]
            log::debug!("{IDEF}: tid {} setup (this is the main thread)", tid());

            start_watchdog();

            if !setting!(gpu) {
                // TODO now: test on linux if gpu acceleration is off
                // supposedly disables gpu acceleration on Linux
                set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
            }

            #[cfg(target_os = "windows")]
            flyout_watch::start();

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

            TrayIconBuilder::with_id(MAIN_TRAY_KEY)
                .menu(&menu)
                .icon(APP_ICON)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    match {
                        tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                        event
                    } {
                        TrayIconEvent::Enter { .. } => {
                            if let Err(e) = ensure_main_window() {
                                log::warn!("{IDEF}: failed to pre-build window: {e}")
                            }
                        }
                        TrayIconEvent::Click {
                            rect,
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } if !recently_closed() => {
                            show_main_window(Some(rect)).unwrap_or_else(|e| {
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
                        show_main_window(anchor)
                            .unwrap_or_else(|e| log::error!("{IDEF}: failed to open window: {e}"));
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => panic!("{IDEF}: there's an unhandled menu option!"),
                })
                .build(app)?;

            // shortcuts
            init_shortcuts()?;

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

// DEBUG

fn tid() -> String {
    format!("{:?}", std::thread::current().id())
}

///
/// Pings the main thread, errors if it doesn't answer within set time
///
/// I see no issues right now, but i suspect my code may be flaky,
/// and i imagine this might be nice to have in user logs.
/// So probably still best kept in production.
///
fn start_watchdog() {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(2));
        let (tx, rx) = std::sync::mpsc::channel();
        if app()
            .run_on_main_thread(move || {
                let _ = tx.send(());
            })
            .is_err()
        {
            log::warn!("{IDEF}: watchdog: run_on_main_thread failed");
            continue;
        }
        if rx.recv_timeout(Duration::from_secs(3)).is_err() {
            log::error!("{IDEF}: watchdog: main thread unresponsive for 3s+");
        }
    });
}
