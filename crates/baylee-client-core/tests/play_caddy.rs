//! `scripts/server/play.caddy` against what the browser client does (#327).
//!
//! The snippet's Content-Security-Policy is a list of hosts the page may
//! fetch from, and the client names its hosts in code. A host the client
//! gains and the policy lacks fails in a player's browser only, as a refused
//! request in the console and a card without its art, so the policy is read
//! here and held to the client's own constants, and the constants to every
//! Scryfall address the client's source spells.

#![allow(clippy::missing_docs_in_private_items)]

use baylee_client_core::images::{SCRYFALL_API, SCRYFALL_BACKS_CDN, SCRYFALL_CDN};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn snippet() -> String {
    std::fs::read_to_string(repo().join("scripts/server/play.caddy")).expect("play.caddy")
}

/// The snippet without its comments: `#` to the end of a line.
fn directives() -> String {
    snippet()
        .lines()
        .map(|line| line.split('#').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The policy's directives, by name.
fn policy() -> Vec<(String, Vec<String>)> {
    let text = directives();
    let line = text
        .lines()
        .find(|l| l.trim_start().starts_with("Content-Security-Policy"))
        .expect("a Content-Security-Policy header");
    let value = line
        .split_once('"')
        .and_then(|(_, rest)| rest.rsplit_once('"'))
        .map(|(value, _)| value)
        .expect("a quoted policy");
    value
        .split(';')
        .filter_map(|d| {
            let mut words = d.split_whitespace().map(str::to_string);
            Some((words.next()?, words.collect()))
        })
        .collect()
}

fn sources(name: &str) -> Vec<String> {
    policy()
        .into_iter()
        .find(|(n, _)| n == name)
        .map_or_else(|| panic!("no {name} in the policy"), |(_, s)| s)
}

/// Every `https://…scryfall…` host spelled in the client's source outside
/// its tests: a file is read up to its first `#[cfg(test)]`, test files and
/// comment lines are skipped.
fn scryfall_hosts_in_source() -> (BTreeSet<String>, usize) {
    let mut hosts = BTreeSet::new();
    let mut files = 0;
    let mut stack = vec![
        repo().join("crates/baylee-client/src"),
        repo().join("crates/baylee-client-core/src"),
    ];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if path.is_dir() {
                if name != "tests" {
                    stack.push(path);
                }
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs")
                || name == "tests.rs"
                || name.ends_with("_tests.rs")
            {
                continue;
            }
            files += 1;
            let text = std::fs::read_to_string(&path).unwrap();
            let lines: Vec<&str> = text.lines().collect();
            for (at, line) in lines.iter().enumerate() {
                // An inline test module (`#[cfg(test)]` then `mod … {`) ends
                // the file's code; `#[cfg(test)] mod tests;` does not.
                if line.contains("#[cfg(test)]")
                    && lines[at + 1..]
                        .iter()
                        .find(|l| !l.trim_start().starts_with("#["))
                        .is_some_and(|l| l.contains("mod ") && l.trim_end().ends_with('{'))
                {
                    break;
                }
                if line.trim_start().starts_with("//") {
                    continue;
                }
                let mut rest: &str = line;
                while let Some(at) = rest.find("https://") {
                    rest = &rest[at + "https://".len()..];
                    let host: String = rest
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
                        .collect();
                    if host.contains("scryfall.") {
                        hosts.insert(host);
                    }
                }
            }
        }
    }
    (hosts, files)
}

fn host(url: &str) -> String {
    url.strip_prefix("https://")
        .expect("an https constant")
        .to_string()
}

#[test]
fn every_scryfall_host_the_client_fetches_from_is_a_constant() {
    let (found, files) = scryfall_hosts_in_source();
    // Both crates together are 184 files (27.09.2026), 353 (09.10.2026, the
    // music's score in modules); a walk that read a handful, or everything
    // twice, is not the walk this test means.
    assert!((150..450).contains(&files), "walked {files} source files");
    let constants: BTreeSet<String> = [SCRYFALL_CDN, SCRYFALL_BACKS_CDN, SCRYFALL_API]
        .into_iter()
        .map(host)
        .collect();
    assert_eq!(
        found, constants,
        "a Scryfall host is spelled outside baylee_client_core::images; \
         name it there and in play.caddy's connect-src"
    );
}

#[test]
fn the_policy_lets_the_page_reach_its_gateway_and_scryfall() {
    let connect = sources("connect-src");
    for url in [SCRYFALL_CDN, SCRYFALL_BACKS_CDN, SCRYFALL_API] {
        assert!(
            connect.iter().any(|s| s == url),
            "connect-src lacks {url}: {connect:?}"
        );
    }
    // The gateway: its HTTP is the page's own origin, and its sockets are
    // named by scheme for browsers whose 'self' does not reach wss.
    assert!(connect.iter().any(|s| s == "'self'"), "{connect:?}");
    assert!(connect.iter().any(|s| s == "wss://{host}"), "{connect:?}");
    // And nothing wider: no scheme or wildcard source.
    for s in &connect {
        assert!(
            !s.contains('*') && !s.ends_with(':'),
            "connect-src opens {s}: {connect:?}"
        );
    }
}

#[test]
fn the_policy_compiles_webassembly_and_evaluates_no_javascript() {
    let script = sources("script-src");
    assert!(
        script.iter().any(|s| s == "'wasm-unsafe-eval'"),
        "{script:?}"
    );
    for refused in ["'unsafe-eval'", "'unsafe-inline'", "*", "data:", "blob:"] {
        assert!(
            !script.iter().any(|s| s == refused),
            "script-src allows {refused}: {script:?}"
        );
    }
    // The page's stylesheet is a file too, and the client styles its one
    // element through the CSSOM (`softkeys.rs`), which a policy allows.
    assert_eq!(sources("style-src"), ["'self'"]);
    assert_eq!(sources("object-src"), ["'none'"]);
    assert_eq!(sources("frame-ancestors"), ["'none'"]);
    assert_eq!(sources("base-uri"), ["'none'"]);
}

/// A page address can carry a seat token (`?game=…&token=…`); an access log
/// would keep it (`docs/privacy.md`).
#[test]
fn the_snippet_keeps_no_access_log() {
    for line in directives().lines() {
        let first = line.split_whitespace().next().unwrap_or("");
        assert_ne!(first, "log", "a log directive: {line}");
        assert_ne!(first, "log_append", "a log directive: {line}");
        assert_ne!(first, "log_name", "a log directive: {line}");
    }
}

/// The snippet serves what the deploy installs, at the path the build is
/// made for.
#[test]
fn the_snippet_serves_what_the_deploy_installs_where_trunk_points() {
    let caddy = directives();
    let deploy =
        std::fs::read_to_string(repo().join("scripts/server/baylee-deploy")).expect("deploy");
    assert!(
        deploy.contains("play=$root/web/play")
            && deploy.contains("root=${BAYLEE_DEPLOY_ROOT:-/opt/baylee}"),
        "the deploy's install directory moved"
    );
    assert!(caddy.contains("root * /opt/baylee/web/play"), "{caddy}");
    assert!(caddy.contains("handle_path /play/*"), "{caddy}");
    assert!(caddy.contains("redir /play /play/"), "{caddy}");
    assert!(
        deploy.contains("--public-url /play/"),
        "trunk builds for another path"
    );
    assert!(
        caddy.contains("header @wasm Content-Type application/wasm"),
        "{caddy}"
    );
    assert!(
        caddy.contains("Cache-Control \"public, max-age=31536000, immutable\""),
        "{caddy}"
    );
}

/// The regular expression the snippet marks content-hashed files with.
fn hashed_pattern() -> String {
    let caddy = directives();
    let line = caddy
        .lines()
        .find(|l| l.trim_start().starts_with("@hashed path_regexp"))
        .expect("an @hashed matcher");
    line.trim_start()
        .trim_start_matches("@hashed path_regexp")
        .trim()
        .to_string()
}

/// Whether `name` matches the snippet's hashed-file pattern. The pattern is
/// `-[0-9a-f]{16}(_bg)?\.(js|wasm|css)$`, held here by the assertion below,
/// so a test with no regex engine can apply it.
fn is_hashed(name: &str) -> bool {
    assert_eq!(
        hashed_pattern(),
        r"-[0-9a-f]{16}(_bg)?\.(js|wasm|css)$",
        "the snippet's pattern changed; change is_hashed with it"
    );
    let Some((stem, ext)) = name.rsplit_once('.') else {
        return false;
    };
    let stem = stem.strip_suffix("_bg").unwrap_or(stem);
    let Some((_, hash)) = stem.rsplit_once('-') else {
        return false;
    };
    matches!(ext, "js" | "wasm" | "css")
        && hash.len() == 16
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[test]
fn the_hashed_pattern_tells_trunks_names_from_the_rest() {
    for hashed in [
        "baylee-client-4dadb4c935a1dc7e.js",
        "baylee-client-4dadb4c935a1dc7e_bg.wasm",
        "page-0123456789abcdef.css",
    ] {
        assert!(is_hashed(hashed), "{hashed}");
    }
    for plain in [
        "index.html",
        "boot.js",
        "baylee-client.js",
        "assets/fonts/AlegreyaSans-Regular.ttf",
        "page-0123456789ABCDEF.css",
        "x-0123456789abcde.js",
    ] {
        assert!(!is_hashed(plain), "{plain}");
    }
}

/// Every inline `<script>` (one without `src`) and every `<style>` in an
/// HTML text, by its opening tag.
fn inline_code(html: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut found = Vec::new();
    for (at, _) in lower.match_indices("<script") {
        let end = lower[at..].find('>').map_or(lower.len(), |e| at + e);
        let tag = &lower[at..end];
        if !tag.contains(" src=") {
            found.push(tag.to_string());
        }
    }
    for (at, _) in lower.match_indices("<style") {
        found.push(lower[at..(at + 20).min(lower.len())].to_string());
    }
    found
}

/// An HTML text without its comments, which may name the tags.
fn markup(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(at) = rest.find("<!--") {
        out.push_str(&rest[..at]);
        rest = rest[at..]
            .find("-->")
            .map_or("", |end| &rest[at + end + 3..]);
    }
    out.push_str(rest);
    out
}

/// The policy allows no inline script or style, so the page has none: the
/// source page, and the tag Trunk writes for the wasm, which loads boot.js.
#[test]
fn the_page_carries_no_inline_script_or_style() {
    let client = repo().join("crates/baylee-client");
    let page = markup(&std::fs::read_to_string(client.join("index.html")).unwrap());
    assert_eq!(inline_code(&page), Vec::<String>::new());
    assert!(page.contains(r#"<link data-trunk rel="copy-file" href="web/boot.js">"#));
    assert!(page.contains(r#"<link data-trunk rel="css" href="web/page.css">"#));

    let trunk = std::fs::read_to_string(client.join("Trunk.toml")).unwrap();
    let pattern = trunk
        .lines()
        .find(|l| l.starts_with("pattern_script"))
        .expect("Trunk.toml sets pattern_script");
    assert_eq!(inline_code(pattern), Vec::<String>::new(), "{pattern}");
    for part in [
        r#"src="{base}boot.js""#,
        r#"data-js="{base}{js}""#,
        r#"data-wasm="{base}{wasm}""#,
    ] {
        assert!(pattern.contains(part), "{pattern} lacks {part}");
    }
    let boot = std::fs::read_to_string(client.join("web/boot.js")).unwrap();
    assert!(boot.contains("script[data-wasm]") && boot.contains("dataset.js"));
}

/// A real `trunk build --release --locked --public-url /play/`, as
/// `baylee-deploy` runs it, checked against the snippet: every file the page
/// names is under `/play/` and in the build, the wasm, its module and the
/// stylesheet carry a hash the snippet caches for good, and the page has no
/// inline code. Ignored: a release wasm build takes minutes (`cargo test -p
/// baylee-client-core --test play_caddy -- --ignored`; trunk and the wasm
/// target installed).
#[test]
#[ignore = "builds the release wasm with trunk"]
fn a_trunk_build_is_what_the_snippet_serves() {
    let dist = std::env::temp_dir().join(format!("baylee-play-dist-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dist);
    let status = std::process::Command::new("trunk")
        .current_dir(repo().join("crates/baylee-client"))
        .args(["build", "index.html", "--release", "--locked"])
        .args(["--public-url", "/play/", "--dist"])
        .arg(&dist)
        .status()
        .expect("trunk runs");
    assert!(status.success(), "trunk build failed");
    let page = std::fs::read_to_string(dist.join("index.html")).unwrap();
    assert_eq!(inline_code(&markup(&page)), Vec::<String>::new(), "{page}");

    let mut named = Vec::new();
    for attr in ["src=\"", "href=\"", "data-js=\"", "data-wasm=\""] {
        for (at, _) in page.match_indices(attr) {
            let value = &page[at + attr.len()..];
            named.push(value[..value.find('"').unwrap()].to_string());
        }
    }
    for url in &named {
        assert!(
            !url.starts_with('/') || url.starts_with("/play/"),
            "{url} is outside /play/"
        );
        // A relative name (the icons) resolves under /play/ from the page.
        let path = url.strip_prefix("/play/").unwrap_or(url);
        assert!(dist.join(path).is_file(), "{url} is not in the build");
    }
    for kind in ["_bg.wasm", ".js", ".css"] {
        assert!(
            named
                .iter()
                .any(|u| u.starts_with("/play/") && u.ends_with(kind) && is_hashed(u)),
            "no hashed {kind} under /play/ in {named:?}"
        );
    }
    assert!(named.iter().any(|u| u == "/play/boot.js"), "{named:?}");
    let _ = std::fs::remove_dir_all(&dist);
}
