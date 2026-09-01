//! End-to-end test for per-account client preferences.
//!
//! Keys and standing orders follow the player, not the machine: signing in on
//! a friend's laptop should bring your keymap and your phase rail with you.
//! The gateway is the locker for that, and deliberately a dumb one — it does
//! not link the client's brain, so it cannot know what a keymap is. What it
//! must do is keep the blob byte-for-byte, keep it per account, and refuse to
//! be used as free storage.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{http, login, spawn_gateway};

#[test]
fn preferences_follow_the_account_and_not_the_machine() {
    let gw = spawn_gateway("settings");
    let token = login(gw.port, "keys@example.com", "rebinder");

    // A player who has never opened the settings screen has no row, and gets
    // an empty object rather than a 404 — the client's own defaults are the
    // right answer, and asking it to tell two failures apart buys nothing.
    let (status, body) = http(gw.port, "GET", "/settings", Some(&token), "");
    assert_eq!(status, 200, "empty settings: {body}");
    assert_eq!(body.trim(), "{}", "expected nothing yet: {body}");

    // What a real client sends: the keymap, both phase rails, the automation
    // flags. The gateway stores it without understanding any of it.
    let prefs = r#"{"keymap":{"confirm":[{"key":"KeyP","shift":true}]},
        "orders":{"skip":[[false,true,false,false,false,false,false,false,false,false,false,false],
                          [false,false,false,false,false,false,false,false,false,false,false,false]]},
        "auto":{"pass_when_nothing_to_do":true}}"#;
    let (status, body) = http(gw.port, "PUT", "/settings", Some(&token), prefs);
    assert_eq!(status, 200, "store settings: {body}");

    let (status, body) = http(gw.port, "GET", "/settings", Some(&token), "");
    assert_eq!(status, 200);
    assert!(
        body.contains("\"KeyP\"") && body.contains("\"pass_when_nothing_to_do\":true"),
        "the preferences did not come back: {body}"
    );

    // A second save replaces rather than merges: the settings screen sends
    // the whole picture, so a field it dropped is a field the player removed.
    let (status, _) = http(
        gw.port,
        "PUT",
        "/settings",
        Some(&token),
        r#"{"auto":{"skip_empty_blocks":true}}"#,
    );
    assert_eq!(status, 200);
    let (_, body) = http(gw.port, "GET", "/settings", Some(&token), "");
    assert!(
        body.contains("skip_empty_blocks") && !body.contains("KeyP"),
        "the old value survived a replacing save: {body}"
    );

    // Not an object: the client would have to guess what to do with it, and
    // a store full of bare numbers is a store nobody can migrate.
    let (status, _) = http(gw.port, "PUT", "/settings", Some(&token), "[1,2,3]");
    assert_eq!(status, 400, "a JSON array was accepted as settings");

    // And not a place to park a megabyte.
    let padding = "x".repeat(32 * 1024);
    let huge = format!("{{\"note\":\"{padding}\"}}");
    let (status, _) = http(gw.port, "PUT", "/settings", Some(&token), &huge);
    assert_eq!(status, 413, "an oversized blob was accepted");
    let (_, body) = http(gw.port, "GET", "/settings", Some(&token), "");
    assert!(
        !body.contains("xxxx"),
        "a refused save still changed the store: {body}"
    );

    // Per account, like the standing answers next door.
    let other = login(gw.port, "other@example.com", "other_player");
    let (status, body) = http(gw.port, "GET", "/settings", Some(&other), "");
    assert_eq!(status, 200);
    assert_eq!(
        body.trim(),
        "{}",
        "one account's keymap leaked into another: {body}"
    );

    // Unauthenticated callers get nothing, and cannot write.
    let (status, _) = http(gw.port, "GET", "/settings", None, "");
    assert_eq!(status, 401, "an anonymous caller read an account's keymap");
    let (status, _) = http(gw.port, "PUT", "/settings", None, "{}");
    assert_eq!(status, 401, "an anonymous caller wrote an account's keymap");
}
