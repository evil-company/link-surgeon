//!
//! Declare settings with the `settings!` macro below.
//!
//! Notes:<br>
//! Originally I disabled gpu by default to save on RAM when open,
//! but keeping it on makes the webview's initial draw basically instant.
//! It's about ~40MB RAM tho literally doesn't matter since it's
//! folded and uses 5MB RAM unless open anyway.

// TODO settings: make sure to load before everything else
// TODO settings: settings rust-to-js sync
// TODO ui pin setting ui: maximize and close from window decor (and replace said decor with custom css) ; and handle runtime change (rebuild?)

use crate::app;
use crate::autostart::toggle_autostart;
use crate::tray::window::toggle_main_window_pin;
use crate::utils::log_err;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use crate::clipboard::toggle_clipboard_watcher;

/// WARN: runs a whole bunch of side effects, make sure their dependents are initialized first
pub fn setup_settings() -> Result<(), String> {
    // TODO settings: load from json or store here
    Settings::current().apply_all();

    Ok(())
}

#[derive(Default)]
pub struct SettingsState(Mutex<Settings>);

macro_rules! settings {
    ($( $name:ident : $ty:ty = $default:expr $(=> $effect:expr)?),* $(,)?) => {
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase", default)]
    pub struct Settings {
        $(pub $name: $ty,)*
    }

    impl Default for Settings {
    fn default() -> Self {
        Self {
            $($name: $default,)*
        }}
    }

    impl Settings {
        fn apply_diff(&self, old: &Settings) {
            $( $(
            if self.$name != old.$name {
                let f: fn(&$ty) = $effect;
                f(&self.$name);
            }
            )? )*
        }

        pub fn apply_all(&self) {
            $( $(
            let f: fn(&$ty) = $effect;
            f(&self.$name);
            )? )*
        }
    }}
}

fn show_requires_reopen_warn() -> tauri::Result<()> {
    app().emit("requires_reopen", true)
}

settings! {
    pin: bool = false => |v| log_err("main window pin", toggle_main_window_pin(*v)), // aka unwidget // TODO ui pin setting ui
    gpu: bool = true => |_| log_err("reopen warn", show_requires_reopen_warn()),
    autostart: bool = true => |v| toggle_autostart(*v),
    clipboard: bool = true => |v| toggle_clipboard_watcher(*v),
    debug_log_foreground_class: bool = false,
}

impl Settings {
    /// WARN: Only use `s` inside `f`, calling `setting!`/`with` in here causes the heat death of the universe (hang)
    pub fn update(f: impl FnOnce(&mut Settings)) {
        let (old, new) = {
            let state = app().state::<SettingsState>();
            let mut guard = state.0.lock().unwrap();
            let old = guard.clone();
            f(&mut guard);
            (old, guard.clone())
        };
        new.apply_diff(&old);
        // TODO settings: js sync
    }
    pub fn with<T>(f: impl FnOnce(&Settings) -> T) -> T {
        f(&app().state::<SettingsState>().0.lock().unwrap())
    }

    pub fn current() -> Settings {
        Self::with(|s| s.clone())
    }
}

/// Use for getting/setting a setting
///
/// WARN: if the new value depends on the old one use `update` directly instead<br> e.g. `Settings::update(|s| s.loaf = !s.loaf)`
#[macro_export]
macro_rules! setting {
    ($field:ident) => {
        $crate::settings::Settings::with(|s| s.$field.clone())
    };
    ($field:ident = $value:expr) => {{
        let v = $value;
        $crate::settings::Settings::update(|s| s.$field = v)
    }};
}

#[tauri::command]
pub fn get_settings() -> Settings {
    Settings::current()
}
#[tauri::command]
pub fn set_settings(patch: serde_json::Value) -> Result<(), String> {
    let mut result = Ok(());
    Settings::update(|s| {
        let mut v = serde_json::to_value(&*s).unwrap();
        json_patch::merge(&mut v, &patch);
        match serde_json::from_value(v) {
            Ok(new) => *s = new,
            Err(e) => result = Err(e.to_string()),
        }
    });
    result
}
