//!
//! not using Tauri's clipboard plugin since it doesn't seem to have events

use crate::surgery::they_did_surgery_on_a;
use crate::utils::BoxErrSendSync;
use crate::{app, setting, IDEF};
use clipboard_rs::{
    Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext,
    ContentFormat, WatcherShutdown,
};
use std::sync::Mutex;
use std::time::Duration;
use tauri::Manager;

/// checks if it's a link (minus trimming)
fn as_single_link(text: &str) -> Option<&str> {
    let t = text.trim();
    if t.is_empty() || t.contains(char::is_whitespace) {
        return None;
    }
    let u = url::Url::parse(t).ok()?;
    matches!(u.scheme(), "http" | "https").then_some(t)
}

const CLIPBOARD_READ_RETRIES: u32 = 3;

/// Reads text from the clipboard.<br>
/// I think it might fail if it's busy so it tries a few times.
fn read_text(ctx: &ClipboardContext) -> Result<String, BoxErrSendSync> {
    for _ in 1..CLIPBOARD_READ_RETRIES {
        if let Ok(t) = ctx.get_text() {
            return Ok(t);
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    Ok(ctx.get_text()?)
}

#[derive(Default)]
pub struct LastClipboardWrite(Mutex<Option<String>>);

struct Handler {
    ctx: ClipboardContext,
}

impl Handler {
    /// treacherous battleground for our link before he can reach zelda,
    /// unworthy links perish, wait-
    fn perform_surgery(&self) -> Result<(), BoxErrSendSync> {
        if !self.ctx.has(ContentFormat::Text) {
            return Ok(());
        }
        for f in [ContentFormat::Image, ContentFormat::Files] {
            if self.ctx.has(f) {
                return Ok(());
            }
        }

        let text = read_text(&self.ctx)?;
        let Some(link) = as_single_link(&text) else {
            #[cfg(debug_assertions)]
            log::debug!("{IDEF}: this is not a link ew");

            return Ok(());
        };

        // prevents our own write
        {
            let state = app().state::<LastClipboardWrite>();
            let mut last = state.0.lock().unwrap_or_else(|e| e.into_inner());
            if last.as_deref() == Some(text.as_str()) {
                *last = None;
                return Ok(());
            }
        }

        let option_tidy = they_did_surgery_on_a(link);
        if option_tidy.is_none() {
            log::debug!("{IDEF}: surgery returned None for {text:?}");
            return Ok(());
        }
        let tidy = option_tidy.unwrap();
        if tidy == text {
            log::debug!("{IDEF}: already tidy {text:?}");
            return Ok(());
        }

        // the user could have copied something while the surgery was going on
        // probably wouldn't happen in practice, our surgeons are very fast
        if self.ctx.get_text().ok().as_deref() != Some(text.as_str()) {
            return Ok(());
        }

        *app()
            .state::<LastClipboardWrite>()
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(tidy.clone());
        self.ctx.set_text(tidy)?;

        #[cfg(debug_assertions)]
        log::debug!("{IDEF}: performed surgery on: {text}");

        Ok(())
    }
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) {
        if !setting!(clipboard) || setting!(hold_clipboard) {
            #[cfg(debug_assertions)]
            log::debug!("{IDEF}: skipped surgery");
            return;
        }
        if let Err(e) = self.perform_surgery() {
            log::warn!(
                "{IDEF}: surgery failed, they probably dropped the scalpel inside oh no: {e}"
            )
        }
    }
}

#[derive(Default)]
pub struct ClipboardShutdown(Mutex<Option<WatcherShutdown>>);

pub fn toggle_clipboard_watcher(enable: bool) {
    let state = app().state::<ClipboardShutdown>();
    let mut slot = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if enable {
        if slot.is_none() {
            *slot = start_clipboard_watcher();
        }
    } else if let Some(shutdown) = slot.take() {
        shutdown.stop();
    }
}

/// watches the clipboard at runtime to run [`ClipboardHandler::on_clipboard_change`]
fn start_clipboard_watcher() -> Option<WatcherShutdown> {
    let ctx = ClipboardContext::new()
        .inspect_err(|e| log::error!("{IDEF}: clipboard context failed: {e}"))
        .ok()?;
    let mut watcher = ClipboardWatcherContext::new()
        .inspect_err(|e| log::error!("{IDEF}: clipboard watcher failed: {e}"))
        .ok()?;
    let shutdown = watcher.add_handler(Handler { ctx }).get_shutdown_channel();

    std::thread::spawn(move || watcher.start_watch());
    Some(shutdown)
}
