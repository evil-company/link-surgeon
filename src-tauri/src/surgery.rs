//!
//! this it the operating room, it's surgin' time

use crate::IDEF;

/// cleans a link of tracking garbage
pub fn they_did_surgery_on_a(link: &str) -> Result<String, String> {
    #[cfg(debug_assertions)]
    log::debug!("{IDEF}: surgeiring");

    Ok(link.parse().unwrap())
}