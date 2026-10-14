//!
//! this it the operating room, it's surgin' time

use crate::{app, IDEF};
use log::kv::Source;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tauri::Manager;

#[derive(Eq, Hash, PartialEq, Copy, Clone)]
enum Site {
    YouTube,
}

struct Rule {
    pattern: Regex,
    replacement: String,
}

static SITE_RULES: LazyLock<HashMap<Site, Vec<Rule>>> = LazyLock::new(|| {
    HashMap::from([(
        Site::YouTube,
        vec![Rule {
            pattern: Regex::new(r"[&,?]si=.+").unwrap(),
            replacement: String::from(""),
        }],
    )])
});

static SITE_MAP: LazyLock<HashMap<Vec<&str>, Site>> =
    LazyLock::new(|| HashMap::from([(vec!["youtube.com", "youtu.be"], Site::YouTube)]));

fn apply_rules(link: &str, site: Site) -> String {
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
    parts[2]
}

/// cleans a link of tracking garbage
pub fn they_did_surgery_on_a(link: &str) -> Option<String> {
    #[cfg(debug_assertions)]
    log::debug!("{IDEF}: surgeiring");

    let parts = link.split("/");
    let collection = parts.collect::<Vec<&str>>();
    let site = identify_site(site_part(collection));
    site?;
    Some(apply_rules(link, site.unwrap()))
}
