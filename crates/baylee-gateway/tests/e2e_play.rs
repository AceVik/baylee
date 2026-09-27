//! `/play` belongs to the web server in front of the gateway, not to the
//! gateway (#327).
//!
//! The browser client is served at `https://<gateway host>/play/` by Caddy
//! (`scripts/server/play.caddy`), which hands every other path to the gateway.
//! Caddy answers `/play` and everything under it before the gateway is asked,
//! so a gateway route there would never be reached behind Caddy and would
//! answer in its place wherever the snippet is missing: either way the two
//! would disagree about a path without a word. So the gateway answers 404 on
//! every one of them, and a route added under `/play` turns this red.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{http, spawn_gateway};

/// What the browser asks for under `/play`: the page, trunk's hashed files,
/// the copied assets, and a path deeper than any of them.
const PATHS: &[&str] = &[
    "/play",
    "/play/",
    "/play/index.html",
    "/play/baylee-client-0123456789abcdef.js",
    "/play/baylee-client-0123456789abcdef_bg.wasm",
    "/play/assets/fonts/AlegreyaSans-Regular.ttf",
    "/play/games/1/ws",
    "/play/a/b/c/d",
];

#[test]
fn the_gateway_answers_nothing_under_play() {
    let gw = spawn_gateway("play");
    for path in PATHS {
        for method in ["GET", "HEAD", "POST"] {
            let (status, body) = http(gw.port, method, path, None, "");
            assert_eq!(status, 404, "{method} {path} answered {status}: {body}");
        }
    }
    // The probe can tell a route from none: the same request one path over
    // is answered.
    let (status, body) = http(gw.port, "GET", "/info", None, "");
    assert_eq!(status, 200, "/info: {body}");
}

/// The routes are registered as string literals in the gateway's source; no
/// literal path starts with `/play`. The probe above covers the paths it
/// names, this covers the ones it does not.
#[test]
fn no_gateway_route_is_spelled_under_play() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack = vec![src];
    let mut read = 0;
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                read += 1;
                for (n, line) in text.lines().enumerate() {
                    // `"/players/{handle}"` is a real route and not one of
                    // these: the path is `/play` itself or below it.
                    assert!(
                        !line.contains("\"/play\"") && !line.contains("\"/play/"),
                        "{}:{}: a path under /play: {line}",
                        path.display(),
                        n + 1
                    );
                }
            }
        }
    }
    // A misread directory would pass every line it never read.
    assert!(read >= 15, "read only {read} source files");
}
