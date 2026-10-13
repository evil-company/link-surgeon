//! TODO linux: linux

use crate::setting;
use std::env::set_var;

/// supposedly disables gpu acceleration on Linux
#[cfg(target_os = "linux")]
pub fn disable_gpu() -> () {
    if !setting!(gpu) {
        // TODO linux: test on linux if gpu acceleration off works
        set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }
}
