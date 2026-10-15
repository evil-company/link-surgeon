//!
//! this it the operating room, it's surgin' time

use crate::{app, setting, IDEF};
use clear_urls::{CleanResult, Error, UrlCleaner};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};
use tauri::Manager;

#[derive(Eq, Hash, PartialEq, Copy, Clone)]
enum Site {
    YouTube,
    Tumblr,
}

struct Rule {
    pattern: Regex,
    replacement: String,
}

static SITE_RULES: LazyLock<HashMap<Site, Vec<Rule>>> = LazyLock::new(|| {
    HashMap::from([
        (
            Site::YouTube,
            vec![Rule {
                pattern: Regex::new(r"[&,?]si=.+").unwrap(),
                replacement: String::from(""),
            }],
        ),
        (
            Site::Tumblr,
            vec![Rule {
                pattern: Regex::new(r"[&,?]source=.+").unwrap(),
                replacement: String::from(""),
            }],
        ),
    ])
});

static SITE_MAP: LazyLock<HashMap<Vec<&str>, Site>> = LazyLock::new(|| {
    HashMap::from([
        (vec!["youtube.com", "youtu.be"], Site::YouTube),
        (vec!["tumblr.com"], Site::Tumblr),
    ])
});

fn apply_rules(link: &str, site: Site) -> String {
    return link.to_string(); // TODO custom strings and opinionated, Autumn
    let mut output = link.to_string();
    let vec = SITE_RULES.get(&site).unwrap();
    for rule in vec.iter() {
        output = rule
            .pattern
            .replace_all(&output, rule.replacement.clone())
            .parse()
            .unwrap();
    }
    output
}

fn identify_site(part: &str) -> Option<Site> {
    for (key, value) in SITE_MAP.iter() {
        if key.contains(&part) {
            return Some(*value);
        }
    }
    None
}

fn site_part(parts: Vec<&str>) -> &str {
    let output = parts[2];
    if (&output[0..4]) == "www." {
        return &output[4..];
    }
    output
}

#[derive(Default)]
pub struct Cleaner(RwLock<Option<UrlCleaner>>);

/// builds/rebuilds the library cleaner
pub fn build_cleaner() -> Result<(), String> {
    let new = UrlCleaner::from_embedded_rules()
        .map_err(|e| e.to_string())?
        .with_skip_local_hosts(setting!(skip_localhosts));
    let state = app().state::<Cleaner>();
    *state.0.write().unwrap_or_else(|e| e.into_inner()) = Some(new);
    Ok(())
}


/// Does the surgery using the crate from crates.io that uses cleanurl's rules
///
/// Our actually experienced surgeon (we're just amateurs)
fn clean_with_lib(link: &str) -> Result<CleanResult, String> {
    let err = format!("{IDEF} cleaner not ready");
    let state = app().try_state::<Cleaner>().ok_or(&err)?;
    let guard = state.0.read().unwrap_or_else(|e| e.into_inner());
    let cleaner = guard.as_ref().ok_or(&err)?;
    Ok(cleaner.clean(link))
}

/// cleans a link of tracking garbage
pub fn they_did_surgery_on_a(link: &str) -> Option<String> {
    #[cfg(debug_assertions)]
    log::debug!("{IDEF}: surgeiring");

    let parts = link.split("/");
    let collection = parts.collect::<Vec<&str>>();
    let site = identify_site(site_part(collection))?;
    let tidy = apply_rules(link, site);
    let clean = match clean_with_lib(&tidy) {
        Ok(c) => c,
        Err(err) => {
            log::error!("{IDEF}: failed to clean with lib: {err}");
            return None;
        }
    };
    Some(clean.url)
}
