//! Stamp CSS before HTML so a stylesheet hash covers its updated dependencies.

use std::fs;
use std::path::Path;

use anyhow::{Result, ensure};
use regex::Regex;
use sha2::{Digest, Sha256};

pub(super) fn digest(path: &Path) -> Result<String> {
    Ok(super::impressum::hex(&Sha256::digest(fs::read(path)?))[..10].to_owned())
}

fn replace(
    text: &str,
    re: &Regex,
    mut one: impl FnMut(&regex::Captures<'_>) -> Result<String>,
) -> Result<String> {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for captures in re.captures_iter(text) {
        let matched = captures.get(0).unwrap();
        out.push_str(&text[last..matched.start()]);
        out.push_str(&one(&captures)?);
        last = matched.end();
    }
    out.push_str(&text[last..]);
    Ok(out)
}

fn asset(site: &Path, relative: &str) -> Result<String> {
    let site = site.canonicalize()?;
    let path = site.join(relative).canonicalize()?;
    ensure!(
        path.starts_with(&site),
        "content stamp asset escapes site directory"
    );
    digest(&path)
}

pub(super) fn css(site: &Path, text: &str) -> Result<String> {
    let re = Regex::new(r#"url\("((?:fonts|img)/[^"?]+)(?:\?v=[0-9a-f]+)?"\)"#)?;
    replace(text, &re, |c| {
        Ok(format!("url(\"{}?v={}\")", &c[1], asset(site, &c[1])?))
    })
}

pub(super) fn html(site: &Path, text: &str) -> Result<String> {
    // Rust regex has no lookbehind/lookahead. Inspect the prefix and data/ in
    // the callback so delimiters shared by adjacent srcset entries stay intact.
    let re = Regex::new(
        r#"/site/([^"\s?,]+\.(?:css|js|woff2|webp|avif|png|jpg|ico))(?:\?v=[0-9a-f]+)?"#,
    )?;
    replace(text, &re, |c| {
        let matched = c.get(0).unwrap();
        let prefix = text[..matched.start()].chars().next_back();
        if !prefix.is_some_and(|p| p == '"' || p == ',' || p.is_whitespace())
            || c[1].starts_with("data/")
        {
            return Ok(matched.as_str().into());
        }
        Ok(format!("/site/{}?v={}", &c[1], asset(site, &c[1])?))
    })
}

pub(super) fn run(root: &Path, check: bool) -> Result<()> {
    let site = root.join("web/site");
    let css_path = site.join("site.css");
    let text = fs::read_to_string(&css_path)?;
    let new_css = css(&site, &text)?;
    let mut stale = Vec::new();
    if text != new_css {
        stale.push("web/site/site.css");
        if !check {
            fs::write(css_path, new_css)?;
        }
    }
    let page = site.join("index.html");
    let text = fs::read_to_string(&page)?;
    if check && !stale.is_empty() {
        stale.push("web/site/index.html");
    } else {
        let new_page = html(&site, &text)?;
        if new_page != text {
            stale.push("web/site/index.html");
            if !check {
                fs::write(page, new_page)?;
            }
        }
    }
    for path in &stale {
        println!("{}: {path}", if check { "stale" } else { "stamped" });
    }
    ensure!(
        !check || stale.is_empty(),
        "stale site content stamps; run site stamp"
    );
    Ok(())
}
