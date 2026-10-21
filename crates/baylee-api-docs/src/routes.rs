//! The routes a crate's routers register, read out of its sources, and the
//! operations a written document holds: the two sets `check` holds equal, so
//! a route registered and never documented (or documented and gone) fails
//! the gate instead of going unnoticed.
//!
//! Both readers are textual, which is enough here: every router is built
//! with `.route("<literal path>", <method router>)` and `.merge`, never
//! `.nest` (the reader refuses one), and oas-forge writes its YAML in one
//! deterministic layout. A reader that misreads finds nothing or everything,
//! so each caller asserts a floor on what it found.

use std::collections::BTreeSet;
use std::path::Path;

/// One operation: an HTTP method (lower case) and a path.
pub type Operation = (String, String);

const METHODS: &[&str] = &["get", "post", "put", "delete", "patch", "options", "head"];

/// Every `.route(...)` registration under `src`, outside test code.
pub fn registered(src: &Path) -> Result<BTreeSet<Operation>, String> {
    let mut found = BTreeSet::new();
    for path in rust_files(src)? {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if text.contains(".nest(") {
            return Err(format!(
                "{} nests a router; the route reader reads only `.route` and `.merge`",
                path.display()
            ));
        }
        let code = without_test_modules(&text);
        let mut rest = code.as_str();
        while let Some(at) = rest.find(".route(") {
            rest = &rest[at + ".route(".len()..];
            let args = balanced(rest);
            let Some(route) = string_literal(args) else {
                continue;
            };
            let methods = &args[args.find(',').unwrap_or(args.len())..];
            for method in METHODS {
                if calls(methods, method) {
                    found.insert(((*method).to_owned(), route.to_owned()));
                }
            }
        }
    }
    Ok(found)
}

/// Every operation in a document oas-forge wrote (`paths:` at two spaces,
/// a path at two, its methods at four).
pub fn documented(yaml: &str) -> BTreeSet<Operation> {
    let mut found = BTreeSet::new();
    let mut in_paths = false;
    let mut path: Option<String> = None;
    for line in yaml.lines() {
        // A blank line is a paragraph inside a description, not a new key.
        if line.trim().is_empty() {
            continue;
        }
        if !line.starts_with(' ') {
            in_paths = line == "paths:";
            path = None;
            continue;
        }
        if !in_paths {
            continue;
        }
        if let Some(key) = line.strip_prefix("  ").filter(|l| !l.starts_with(' ')) {
            path = key
                .strip_suffix(':')
                .map(|p| p.trim_matches('"').to_owned());
        } else if let (Some(key), Some(path)) = (
            line.strip_prefix("    ").filter(|l| !l.starts_with(' ')),
            &path,
        ) && let Some(method) = key.strip_suffix(':').filter(|m| METHODS.contains(m))
        {
            found.insert((method.to_owned(), path.clone()));
        }
    }
    found
}

/// Every channel address in a `*.asyncapi.yaml` oas-forge wrote, as the
/// `GET` that opens its socket (`channels:` at the top, a channel at two
/// spaces, its `address` at four).
pub fn channels(yaml: &str) -> BTreeSet<Operation> {
    let mut found = BTreeSet::new();
    let mut in_channels = false;
    for line in yaml.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if !line.starts_with(' ') {
            in_channels = line == "channels:";
            continue;
        }
        if in_channels && let Some(address) = line.strip_prefix("    address: ") {
            found.insert(("get".to_owned(), address.trim_matches('"').to_owned()));
        }
    }
    found
}

fn rust_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let path = entry.map_err(|e| e.to_string())?.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if path.is_dir() {
                if name != "tests" {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "rs")
                && name != "tests.rs"
                && !name.ends_with("_tests.rs")
            {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// The text with every `#[cfg(test)]` item's braces removed, so a router a
/// test builds is not read as one the service serves.
fn without_test_modules(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("#[cfg(test)]") {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        match rest.find(['{', ';']) {
            Some(end) if rest.as_bytes()[end] == b'{' => {
                let body = balanced_braces(&rest[end + 1..]);
                rest = &rest[end + 1 + body.len()..];
            }
            Some(end) => rest = &rest[end + 1..],
            None => rest = "",
        }
    }
    out.push_str(rest);
    out
}

/// The text up to the parenthesis that closes the one already opened.
fn balanced(text: &str) -> &str {
    let mut depth = 1_u32;
    for (i, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return &text[..i];
                }
            }
            _ => {}
        }
    }
    text
}

/// The text up to and including the brace that closes the one already opened.
fn balanced_braces(text: &str) -> &str {
    let mut depth = 1_u32;
    for (i, c) in text.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &text[..=i];
                }
            }
            _ => {}
        }
    }
    text
}

/// The first string literal in `text`, if it comes before anything else.
fn string_literal(text: &str) -> Option<&str> {
    let text = text.trim_start();
    let inner = text.strip_prefix('"')?;
    Some(&inner[..inner.find('"')?])
}

/// Whether `text` calls `method(` as a method router (`get(`, `.post(`,
/// `routing::put(`), and not as the tail of a longer name.
fn calls(text: &str, method: &str) -> bool {
    let needle = format!("{method}(");
    text.match_indices(&needle).any(|(at, _)| {
        text[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_route_with_two_methods_is_two_operations_and_a_test_router_none() {
        let code = r#"
            Router::new()
                .route("/decks", get(list).post(create))
                .route(
                    "/decks/{id}",
                    routing::delete(drop_deck),
                );
            #[cfg(test)]
            mod tests { fn r() { Router::new().route("/test-only", get(x)); } }
        "#;
        let dir = std::env::temp_dir().join(format!("api-docs-routes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("lib.rs"), code).unwrap();
        let found = registered(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let want: BTreeSet<Operation> = [
            ("get", "/decks"),
            ("post", "/decks"),
            ("delete", "/decks/{id}"),
        ]
        .iter()
        .map(|(m, p)| ((*m).to_owned(), (*p).to_owned()))
        .collect();
        assert_eq!(found, want);
    }

    #[test]
    fn the_document_reader_takes_methods_under_paths_only() {
        let yaml = "openapi: 3.1.0\npaths:\n  /a:\n    get:\n      description: |-\n        One.\n\n        Two.\n    parameters: []\n  \"/b/{id}\":\n    post:\n      x: 1\ncomponents:\n  schemas:\n    get:\n      type: object\n";
        let found = documented(yaml);
        let want: BTreeSet<Operation> = [("get", "/a"), ("post", "/b/{id}")]
            .iter()
            .map(|(m, p)| ((*m).to_owned(), (*p).to_owned()))
            .collect();
        assert_eq!(found, want);
    }

    #[test]
    fn a_channel_is_the_get_that_opens_its_socket() {
        let yaml = "asyncapi: 3.0.0\nchannels:\n  lobby:\n    address: /lobby/ws\n    description: |-\n      One.\n\n      Two.\n    messages: {}\n  seat:\n    address: \"/games/{id}/ws\"\noperations:\n  x:\n    address: /not-a-channel\n";
        let found = channels(yaml);
        let want: BTreeSet<Operation> = [("get", "/lobby/ws"), ("get", "/games/{id}/ws")]
            .iter()
            .map(|(m, p)| ((*m).to_owned(), (*p).to_owned()))
            .collect();
        assert_eq!(found, want);
    }
}
