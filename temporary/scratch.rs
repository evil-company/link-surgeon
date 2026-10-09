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
    };
}

// const SITE_MAP: HashMap<&str[], Site> = HashMap::from([
//
// ]);

fn apply_rules(link: &str, site: Site) -> &str {

}

fn identify_site(part: &str) {}

fn site_part(parts: Vec<&str>) -> &str {
    return parts[2];
}

fn surgery(link: &str) {
    let parts = link.split("/");
    let collection = parts.collect::<Vec<&str>>();
}