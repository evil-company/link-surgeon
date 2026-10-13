//! Originally I disabled gpu by default to save on RAM when open,
//! but keeping it on makes the webview's initial draw basically instant.
//! It's about ~40MB RAM tho literally doesn't matter since it's
//! folded and uses 5MB RAM unless open anyway.

// TODO now: settings json ; or tauri stor
// TODO now: settings rust-to-js sync

use crate::app;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::Manager;

#[derive(Default)]
pub struct SettingsState(Mutex<Settings>);

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub pin: bool, // TODO ui: pin setting ui
    pub gpu: bool,
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
    pub fn with<T>(f: impl FnOnce(&Settings) -> T) -> T {
        f(&app().state::<SettingsState>().0.lock().unwrap())
    }

    pub fn current() -> Settings {
        Self::with(|s| s.clone())
    }
}

/// use for getting a setting
#[macro_export]
macro_rules! setting {
    ($field:ident) => {
        $crate::settings::Settings::with(|s| s.$field.clone())
    };
}
