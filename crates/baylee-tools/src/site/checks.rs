//! Offline site checks, shared by `site check` and the migrated regression tests.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, ensure};
use regex::Regex;
use serde_json::Value;

use super::{bulk, html, impressum, stamp};

pub(super) const PAGES: [&str; 3] = ["index.html", "impressum.html", "datenschutz.html"];
pub(super) const EMAIL: &str = r"[\w.+-]+@[\w-]+\.[\w.-]+";
pub(super) const DIGITS: &str = r"\d{6,}";

pub(super) fn read(root: &Path, name: &str) -> Result<String> {
    Ok(fs::read_to_string(root.join("web/site").join(name))?)
}

pub(super) fn page(root: &Path, name: &str, check: &str) -> Result<()> {
    let tree = html::parse(&read(root, name)?)?;
    for (i, n) in tree.nodes.iter().enumerate() {
        match check {
            "csp" => {
                ensure!(
                    !n.attrs.contains_key("style")
                        && !n.attrs.keys().any(|a| a.starts_with("on"))
                        && n.tag != "style",
                    "{name}: inline style or handler"
                );
                ensure!(
                    n.tag != "script" || n.text.trim().is_empty(),
                    "{name}: inline script"
                );
            }
            "alt" if n.tag == "img" => {
                ensure!(n.attrs.contains_key("alt"), "{name}: picture without alt");
            }
            "label"
                if ["input", "textarea", "select"].contains(&n.tag.as_str())
                    && n.attr("type") != "hidden" =>
            {
                let wrapped = tree.ancestors(i).iter().any(|a| a.tag == "label");
                let label = !n.attr("id").is_empty()
                    && tree
                        .nodes
                        .iter()
                        .any(|l| l.tag == "label" && l.attr("for") == n.attr("id"));
                ensure!(
                    wrapped
                        || label
                        || !n.attr("aria-label").is_empty()
                        || !n.attr("aria-labelledby").is_empty(),
                    "{name}: control without label"
                );
            }
            "button" => {
                if n.tag == "button" {
                    ensure!(
                        ["button", "submit"].contains(&n.attr("type"))
                            && (!tree.text(i).is_empty() || !n.attr("aria-label").is_empty()),
                        "{name}: button lacks type or name"
                    );
                }
                if n.tag == "dialog" {
                    ensure!(
                        !n.attr("aria-label").is_empty() || !n.attr("aria-labelledby").is_empty(),
                        "{name}: unnamed dialog"
                    );
                }
            }
            "german" if n.class("de") => {
                ensure!(n.attr("lang") == "de", "{name}: German block lacks lang=de");
            }
            "noopener" if n.tag == "a" && n.attr("target") == "_blank" => ensure!(
                n.attr("rel").contains("noopener"),
                "{name}: blank target lacks noopener"
            ),
            _ => {}
        }
    }
    if check == "headings" {
        headings(&tree, name)?;
    }
    if check == "footer" {
        let foot: Vec<_> = tree
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.tag == "footer")
            .collect();
        ensure!(foot.len() == 1, "{name}: expected one footer");
        let hrefs: BTreeSet<_> = tree
            .descendants(foot[0].0)
            .filter(|n| n.tag == "a")
            .map(|n| n.attr("href"))
            .collect();
        ensure!(
            hrefs.contains("/site/datenschutz.html") && hrefs.contains("/site/impressum.html"),
            "{name}: footer lacks legal pages"
        );
    }
    Ok(())
}

fn headings(tree: &html::Tree, name: &str) -> Result<()> {
    for other in ["de", "en"] {
        let levels: Vec<_> = tree
            .nodes
            .iter()
            .enumerate()
            .filter(|(i, n)| !n.class(other) && !tree.ancestors(*i).iter().any(|a| a.class(other)))
            .filter_map(|(_, n)| {
                let b = n.tag.as_bytes();
                (b.len() == 2 && b[0] == b'h' && (b'1'..=b'6').contains(&b[1])).then(|| b[1] - b'0')
            })
            .collect();
        ensure!(
            levels.first() == Some(&1)
                && levels.iter().rposition(|v| *v == 1) == Some(0)
                && levels.windows(2).all(|p| p[1] <= p[0] + 1),
            "{name}: invalid heading sequence"
        );
    }
    Ok(())
}

pub(super) fn picture_css(root: &Path) -> Result<()> {
    let css = read(root, "site.css")?;
    let css = Regex::new(r"(?s)/\*.*?\*/")?.replace_all(&css, "");
    let selectors = Regex::new(r"\.(ctile-pic|cp-pic)\b")?;
    let forbidden = Regex::new(r"(^|;|\s)(filter|opacity|mix-blend-mode|backdrop-filter)\s*:")?;
    for c in Regex::new(r"([^{}]+)\{([^{}]*)\}")?.captures_iter(&css) {
        if selectors.is_match(&c[1]) {
            ensure!(
                !forbidden.is_match(&c[2]),
                "card picture is filtered, faded or blended"
            );
        }
    }
    let rule = css
        .split(".ctile-pic img")
        .nth(1)
        .and_then(|s| s.split('}').next())
        .context("missing card image fit rule")?;
    ensure!(
        rule.contains("object-fit: contain"),
        "card image must fit completely"
    );
    ensure!(
        !Regex::new(r"\.style\.(filter|opacity)\s*=.*(ctile|cp-pic)")?
            .is_match(&read(root, "site.js")?),
        "card picture modified in script"
    );
    Ok(())
}

pub(super) fn privacy(root: &Path, check: &str) -> Result<()> {
    let caddy = fs::read_to_string(root.join("scripts/server/site.caddy"))?;
    match check {
        "hosts" => {
            let csp_re = Regex::new(r#"Content-Security-Policy "([^"]+)""#)?;
            let csp = csp_re.captures(&caddy).context("missing CSP")?;
            let notice = read(root, "datenschutz.html")?;
            for c in Regex::new(r"https://([a-z0-9.-]+)")?.captures_iter(&csp[1]) {
                ensure!(
                    c[1].ends_with(".baylee.app") || notice.contains(&c[1]),
                    "foreign CSP host missing from privacy notice"
                );
            }
        }
        "cookie" => {
            for entry in fs::read_dir(root.join("web/site"))? {
                let path = entry?.path();
                if path.extension().is_some_and(|s| s == "js") {
                    ensure!(
                        !fs::read_to_string(path)?.contains("document.cookie"),
                        "site script sets a cookie"
                    );
                }
            }
        }
        "log" => {
            let re = Regex::new(r"^(log|log_skip|skip_log)\b")?;
            ensure!(
                !caddy
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.starts_with('#'))
                    .any(|l| re.is_match(l)),
                "site Caddy snippet writes an access log"
            );
        }
        "controller" => {
            let tree = html::parse(&read(root, "datenschutz.html")?)?;
            let i = tree
                .nodes
                .iter()
                .position(|n| n.attr("id") == "verantwortlich")
                .context("missing controller section")?;
            ensure!(
                tree.descendants(i)
                    .any(|n| n.tag == "a" && n.attr("href") == "/site/impressum.html"),
                "controller section lacks Impressum link"
            );
            let text = tree.text(0);
            ensure!(
                !Regex::new(EMAIL)?.is_match(&text)
                    && !Regex::new(r"\+\d[\d ]{6,}")?.is_match(&text),
                "private contact data in privacy notice"
            );
        }
        _ => unreachable!(),
    }
    Ok(())
}

pub(super) fn committed(root: &Path, check: &str) -> Result<()> {
    let mut paths: Vec<_> = fs::read_dir(root.join("web/site/data/sets"))?
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().is_some_and(|n| {
                let n = n.to_string_lossy();
                n.starts_with("set-") && n.ends_with(".json")
            })
        })
        .collect();
    paths.sort();
    if check == "exists" {
        ensure!(paths.len() > 100, "committed set files missing");
    }
    let mut seen = 0;
    let mut ub_seen = 0;
    for path in paths {
        let name = path.file_name().unwrap().to_string_lossy();
        let code = &name[4..name.len() - 5];
        let doc: Value = serde_json::from_reader(std::io::BufReader::new(fs::File::open(&path)?))?;
        if bulk::UB_SETS.contains(&code) {
            ub_seen += 1;
        }
        if check == "code" {
            ensure!(doc["code"] == code, "set file names a different code");
        }
        for c in doc["cards"].as_array().context("set has no cards array")? {
            let img = c.get("img");
            match check {
                "absent" if c["status"] == "absent" => {
                    seen += 1;
                    ensure!(img.is_none(), "absent card pictured");
                }
                "ub" if bulk::UB_SETS.contains(&code) => {
                    ensure!(img.is_none(), "Universes Beyond set pictured");
                }
                "uuid" if img.is_some() => {
                    seen += 1;
                    ensure!(
                        img.and_then(Value::as_str).is_some_and(bulk::valid_uuid),
                        "invalid printing UUID"
                    );
                }
                "status" if img.is_some() => ensure!(
                    bulk::PICTURED.contains(&c["status"].as_str().unwrap_or("")),
                    "ineligible card status pictured"
                ),
                "facts" => {
                    ensure!(
                        (c.get("uri").is_none() && c.get("artist").is_none()) || img.is_some(),
                        "printing facts without picture"
                    );
                    if let Some(uri) = c.get("uri") {
                        ensure!(
                            uri.as_str()
                                .is_some_and(|u| u.starts_with("https://scryfall.com/card/")
                                    && !u.contains('?')),
                            "invalid Scryfall card page"
                        );
                    }
                }
                _ => {}
            }
        }
    }
    if ["absent", "uuid"].contains(&check) {
        ensure!(seen > 0, "committed data check was vacuous");
    }
    if check == "ub" {
        ensure!(ub_seen > 0, "no Universes Beyond set checked");
    }
    Ok(())
}

pub(super) fn private_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn walk(root: &Path, private: bool, out: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let ty = entry.file_type()?;
            if ty.is_dir() {
                if name != "node_modules" && name != ".git" {
                    walk(&entry.path(), private || name == "private", out)?;
                }
            } else if (ty.is_file() || (ty.is_symlink() && entry.path().is_file()))
                && (private
                    || name.ends_with(".bin")
                    || (name.starts_with("impressum-data") && name.ends_with(".json")))
            {
                out.push(entry.path());
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, false, &mut out)?;
    out.sort();
    Ok(out)
}

pub(super) fn ignores(root: &Path) -> Result<()> {
    for path in [
        "web/site/private/impressum.bin",
        "web/impressum-data.json",
        "impressum-data.json",
    ] {
        let status = Command::new("git")
            .current_dir(root)
            .args(["check-ignore", "-q", path])
            .status()?;
        ensure!(status.success(), "private path not gitignored");
    }
    let status = Command::new("git")
        .current_dir(root)
        .args(["check-ignore", "-q", "web/site/impressum.html"])
        .status()?;
    ensure!(
        status.code() == Some(1),
        "Impressum page must not be ignored"
    );
    Ok(())
}

// Each branch is a separate static guard, also exercised independently in tests.
#[allow(clippy::too_many_lines)]
pub(super) fn impressum(root: &Path, check: &str) -> Result<()> {
    let js = read(root, "impressum.js")?;
    let html = read(root, "impressum.html")?;
    let css = read(root, "site.css")?;
    match check {
        "constants" => {
            let re = Regex::new(r#"var SCRIPT_KEY = "([0-9a-fA-F]{32})";"#)?;
            ensure!(
                re.captures(&js).context("browser key missing")?[1].to_lowercase()
                    == impressum::hex(&impressum::SCRIPT_KEY),
                "browser Impressum key differs"
            );
            let service = fs::read_to_string(root.join("crates/baylee-feedback/src/impressum.rs"))?;
            let re = Regex::new(r#"pub const SCRIPT_KEY: &str = "([0-9a-fA-F]{32})";"#)?;
            ensure!(
                re.captures(&service).context("feedback key missing")?[1].to_lowercase()
                    == impressum::hex(&impressum::SCRIPT_KEY),
                "feedback Impressum key differs"
            );
            for (key, value) in [
                ("NONCE_LEN", impressum::NONCE_LEN.to_string()),
                ("VERSION", impressum::VERSION.to_string()),
            ] {
                let re = Regex::new(&format!(r"var {key} = (\d+);"))?;
                ensure!(
                    re.captures(&js).context("browser constant missing")?[1] == value,
                    "browser Impressum constant differs"
                );
            }
            ensure!(
                js.contains("var MAGIC = \"BYIM\";"),
                "browser Impressum magic differs"
            );
        }
        "shapes" => {
            let email = Regex::new(EMAIL)?;
            let digits = Regex::new(DIGITS)?;
            for text in [&js, &html] {
                ensure!(
                    !email.is_match(text) && !digits.is_match(text),
                    "private data shape in static Impressum file"
                );
            }
        }
        "page" => {
            for needle in [
                "§ 5 DDG",
                "Bitte JavaScript aktivieren",
                "Kontaktformular",
                "noindex",
                "/site/impressum.js",
            ] {
                ensure!(
                    html.contains(needle),
                    "Impressum page missing required content"
                );
            }
        }
        "honeypot" => {
            let tree = html::parse(&html)?;
            let links: Vec<_> = tree
                .nodes
                .iter()
                .filter(|n| n.tag == "a" && n.attr("href") == "/site/impressum/kontakt")
                .collect();
            ensure!(
                links.len() == 1
                    && links[0].attr("tabindex") == "-1"
                    && links[0].attr("aria-hidden") == "true"
                    && links[0].class("hp"),
                "honeypot link is visible or duplicated"
            );
            let re = Regex::new(r"(^|[^\w-])\.hp\s*\{([^{}]*)\}")?;
            let hidden = Regex::new(r"display\s*:\s*none")?;
            ensure!(
                re.captures_iter(&css).any(|c| hidden.is_match(&c[2])),
                "honeypot CSS not hidden"
            );
            ensure!(
                !html.contains("/private/") && !js.contains("/private/"),
                "old private path exposed"
            );
        }
        "links" => ensure!(
            read(root, "index.html")?
                .matches("\"/site/impressum.html\"")
                .count()
                >= 2,
            "index lacks Impressum links"
        ),
        "orders" => {
            let re = Regex::new(r"\.o(\d+)\s*\{\s*order:\s*(\d+)\s*;?\s*\}")?;
            let found: BTreeMap<usize, usize> = re
                .captures_iter(&css)
                .map(|c| Ok((c[1].parse()?, c[2].parse()?)))
                .collect::<Result<_>>()?;
            ensure!(
                found == (0..impressum::MAX_LEN).map(|i| (i, i)).collect(),
                "Impressum CSS order classes differ"
            );
            ensure!(
                Regex::new(r"(^|[^\w-])\.o\d+\s*\{")?
                    .find_iter(&css)
                    .count()
                    == impressum::MAX_LEN,
                "duplicate Impressum order classes"
            );
        }
        "decoys" => {
            let re = Regex::new(r"([^{}]*\.scr-x[^{}]*)\{([^{}]*)\}")?;
            let forbidden = Regex::new(r"display\s*:\s*none|visibility\s*:\s*hidden")?;
            let rules: Vec<_> = re.captures_iter(&css).collect();
            ensure!(
                !rules.is_empty() && rules.iter().all(|c| !forbidden.is_match(&c[2])),
                "decoys removed from innerText"
            );
        }
        "robots" => {
            let robots = read(root, "robots.txt")?;
            let lines: BTreeSet<_> = robots.lines().map(str::trim).collect();
            ensure!(
                lines.contains("Disallow: /site/impressum.html")
                    && lines.contains("Disallow: /site/impressum/")
                    && !lines.contains("Disallow: /private/"),
                "robots exposes Impressum routes"
            );
        }
        _ => unreachable!(),
    }
    Ok(())
}

pub(super) fn run(root: &Path) -> Result<()> {
    for name in PAGES {
        for check in [
            "csp", "alt", "label", "button", "german", "headings", "noopener", "footer",
        ] {
            page(root, name, check)?;
        }
    }
    picture_css(root)?;
    for check in ["hosts", "cookie", "log", "controller"] {
        privacy(root, check)?;
    }
    for check in ["exists", "code", "absent", "ub", "uuid", "status", "facts"] {
        committed(root, check)?;
    }
    ensure!(
        private_files(&root.join("web"))?.is_empty(),
        "private file present under web"
    );
    ignores(root)?;
    for check in [
        "constants",
        "shapes",
        "page",
        "honeypot",
        "links",
        "orders",
        "decoys",
        "robots",
    ] {
        impressum(root, check)?;
    }
    stamp::run(root, true)?;
    println!("site catalog, pages, privacy and stamps checked offline");
    Ok(())
}
