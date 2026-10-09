type Site = "youtube" //| "facebook" | "xtwitter" | "instagram" | "bluesky" | "amazon" etc

const siteMap: Map<string[], Site> = new Map([
    [["youtube.com", "youtu.be"], "youtube"]
]);

type Rule = {
    pattern: RegExp,
    replacement: string
}

const siteRules: Record<Site, Rule[]> = {
    "youtube": [
        {
            pattern: /[&,?]si=.+/,
            replacement: ""
        },
        {
            pattern: /youtu.be\//,
            replacement: "youtube.com/watch?v="
        }
    ]
}

const applyRules = (link: string, site: Site) => {
    for (const rule of siteRules[site]) {
        link = link.replace(rule.pattern, rule.replacement);
    }
    return link;
};

const identifySite = (part: string) => {
    for (const key of siteMap.keys()) if (key.includes(part)) return siteMap.get(key);
    return null;
};

const sitePart = (parts: string[]) => parts[2];

const surgery = (link: string) => {
    const parts = link.split("/");
    const site = identifySite(sitePart(parts));
    if(!site) return;
    const cleanedLink = applyRules(link, site);
    console.log("original link", link)
    console.log("cleaned link", cleanedLink);
};

surgery("https://youtu.be/jQj-8E09_xs?si=gll5jknlYGbfcgar");