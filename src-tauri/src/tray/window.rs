//!
//! Some of this is kind of overkill and me (Leth) having fun with opening the app as fast as possible,
//! please enjoy debugging this >:).

pub const MAIN_WINDOW_KEY: &str = "link-surgeon";
const TASKBAR_MARGIN: i32 = 10;
// px
const BLUR_CLOSE_DEBOUNCE: Duration = Duration::from_millis(200);
const PREOPEN_TIMEOUT: Duration = Duration::from_secs(24);

use crate::{app, setting, FULL_APP_NAME};
use crate::debug::get_thread_id;
#[cfg(target_os = "windows")]
use crate::platform::windows::{set_windows_window_builder_settings, set_windows_window_settings};
use crate::IDEF;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::webview::{Color, PageLoadEvent};
use tauri::{
    Manager, PhysicalPosition, Rect, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

/// positions the window right above taskbar and clamps it,
/// horizontally centered to the `rect` (tray icon here)
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
    let x = (icon_center - win.width as i32 / 2)
        .min(right - win.width as i32)
        .max(left);
    let y = (bottom - win.height as i32 - bottom_margin).max(area.position.y);

    window.set_position(PhysicalPosition::new(x, y))
}

#[derive(Default)]
pub struct LastShown(Mutex<Option<Instant>>);
#[derive(Default)]
pub struct LastWanted(Mutex<Option<Instant>>);

/// prevents the user from double-clicking and spam clicking the window
fn recently_shown() -> bool {
    app()
        .state::<LastShown>()
        .0
        .lock()
        .unwrap()
        .is_some_and(|t| t.elapsed() < BLUR_CLOSE_DEBOUNCE)
}

/// why did i name it like that again
fn touch_wanted() {
    *app().state::<LastWanted>().0.lock().unwrap() = Some(Instant::now());
}

fn idle_for() -> Duration {
    app()
        .state::<LastWanted>()
        .0
        .lock()
        .unwrap()
        .map_or(Duration::MAX, |t| t.elapsed())
}

#[derive(Default)]
struct GateInner {
    loaded: bool,
    pending: Option<Option<Rect>>,
}

#[derive(Default)]
pub struct PageGate(Mutex<GateInner>);

/// Places and shows the window which should be pre-built by this point.<br>
/// Skips if window doesn't exists.
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
    window.unminimize()?; // prevents getting stuck in pin mode
    window.set_focus()?;
    Ok(())
}

/// closes the window if unfocused
fn handle_window_focus(w: &WebviewWindow) -> () {
    if setting!(pin) || !w.is_visible().unwrap_or(false) {
        return;
    }
    if recently_shown() {
        let _ = w.set_focus();
        return;
    }
    let _ = w.close();
}

/// Closes the window if it's been long enough since opening tray/hover.
/// This is mostly just a nit in case the user unfocuses or something in a way we could not predict
fn handle_preopen_timeout() -> () {
    std::thread::spawn(|| loop {
        let idle = idle_for();
        if idle < PREOPEN_TIMEOUT {
            std::thread::sleep(PREOPEN_TIMEOUT - idle);
            continue;
        }
        if let Some(w) = app().get_webview_window(MAIN_WINDOW_KEY) {
            if !w.is_visible().unwrap_or(true) {
                let _ = w.close();
            }
        }
        return;
    });
}

/// allows the window to get revealed and reveals it
fn handle_page_load() -> () {
    let exists = app().get_webview_window(MAIN_WINDOW_KEY).is_some();
    let pending = {
        let gate = app().state::<PageGate>();
        let mut g = gate.0.lock().unwrap();
        g.loaded = true;
        if exists {
            g.pending.take()
        } else {
            None
        }
    };
    if let Some(anchor) = pending {
        let _ = reveal(anchor);
    }
}

fn build_main_window() -> tauri::Result<()> {
    // build config
    let builder = WebviewWindowBuilder::new(app(), MAIN_WINDOW_KEY, WebviewUrl::default())
        .title(FULL_APP_NAME)
        .inner_size(400., 600.)
        .min_inner_size(400., 400.)
        .background_color(Color(47, 47, 47, 255)) // TODO ui: make flash background color match ui
        .skip_taskbar(true)
        .always_on_top(true)
        .resizable(setting!(pin))
        .visible(false)
        .decorations(setting!(pin))
        .on_page_load(|_, payload| {
            if let PageLoadEvent::Finished = payload.event() {
                handle_page_load();
            }
        });

    #[cfg(target_os = "windows")]
    let builder = set_windows_window_builder_settings(builder);

    let window = builder.build()?;

    #[cfg(target_os = "windows")]
    set_windows_window_settings(&window);
    // TODO other maybe: maybe close tray immediately instead of window's delay as a minor aesthetic nit

    // window events
    let w = window.clone();
    window.on_window_event(move |e| {
        if let WindowEvent::Focused(false) = e {
            handle_window_focus(&w);
        }
    });

    // prebuilds on hover, closes if timed out
    handle_preopen_timeout();

    Ok(())
}

#[derive(Default)]
pub struct Building(AtomicBool);

/// builds the window if it's not built/being built
pub fn ensure_main_window() -> tauri::Result<()> {
    touch_wanted();
    if app().get_webview_window(MAIN_WINDOW_KEY).is_some() {
        return Ok(());
    }

    if app().state::<Building>().0.swap(true, Ordering::SeqCst) {
        log::debug!(
            "{IDEF}: {} ensure: skipped, build already running",
            get_thread_id()
        );
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

pub fn show_main_window(anchor: Option<Rect>) -> tauri::Result<()> {
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
