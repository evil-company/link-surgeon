use std::collections::HashMap;
use std::sync::LazyLock;
use regex::Regex;
use lazy_static::lazy_static;

fn main() {
    println!("Hello, world!");
}

enum Site {
    YouTube
}

struct Rule {
    pattern: Regex,
    replacement: String
}

lazy_static! {
    static ref SITE_RULES: HashMap<Site, Vec<Rule>> = {
        let mut m = HashMap::from([
            (crate::Site::YouTube, [
                Rule {
                    pattern: Regex::new(r"[&,?]si=.+").unwrap(),
                    replacement: String::from("")
                }, Rule {
                    pattern: Regex::new(r"youtu.be/").unwrap(),
                    replacement: String::from("youtube.com/watch?v=")
                }
            ])
        ]);
        m
    };
}

lazy_static! {
    static ref SITE_MAP: HashMap<Vec<&str>, Site> = {
        let mut m = HashMap::from([
            (["youtube.com", "youtu.be"], crate::Site::YouTube)
        ]);
        m
    }
}

// fn apply_rules(link: &str, site: Site) -> &str {
//
// }

fn identify_site(part: &str) -> Option<Site> {
    for (key, value) in SITE_MAP {
        if (key.contains(part)) {
            return value;
        }
    }
    return false;
}

fn site_part(parts: Vec<&str>) -> &str {
    return parts[2];
}

fn surgery(link: &str) {
    let parts = link.split("/");
    let collection = parts.collect::<Vec<&str>>();
}