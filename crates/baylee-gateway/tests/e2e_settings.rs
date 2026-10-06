//! End-to-end test for per-account client preferences.
//!
//! Keys and standing orders follow the player, not the machine: signing in on
//! a friend's laptop should bring your keymap and your phase rail with you.
//! The gateway is the locker for that, and deliberately a dumb one — it does
//! not link the client's brain, so it cannot know what a keymap is. What it
//! must do is keep each top-level key's value as it was sent, merge a save
//! into what is there rather than replace it (so a client that does not know
//! a key cannot erase it), keep it per account, and refuse to be used as free
//! storage.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{http, login, spawn_gateway};

/// `PUT /settings` as `token`, for its status.
fn put(port: u16, token: &str, body: &str) -> u16 {
    http(port, "PUT", "/settings", Some(token), body).0
}

/// `GET /settings` as `token`, for its body.
fn get(port: u16, token: &str) -> String {
    http(port, "GET", "/settings", Some(token), "").1
}

#[test]
fn preferences_follow_the_account_and_not_the_machine() {
    let gw = spawn_gateway("settings");
    let token = login(gw.port, "keys", "rebinder");

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

    // A second save merges per top-level key (M4-7): the key it names is
    // replaced whole — the old `auto` is gone, not deep-merged — and the
    // keys it does not name are kept.
    let status = put(gw.port, &token, r#"{"auto":{"skip_empty_blocks":true}}"#);
    assert_eq!(status, 200);
    let body = get(gw.port, &token);
    assert!(
        body.contains("skip_empty_blocks") && !body.contains("pass_when_nothing_to_do"),
        "a named key was merged into rather than replaced: {body}"
    );
    assert!(
        body.contains("\"KeyP\"") && body.contains("\"orders\""),
        "a save that named only `auto` erased the keys it did not name: {body}"
    );

    // An explicit `null` is how a key goes; a nested `null` is a value.
    let status = put(
        gw.port,
        &token,
        r#"{"orders":null,"shell_keys":{"search":null}}"#,
    );
    assert_eq!(status, 200);
    let body = get(gw.port, &token);
    assert!(
        !body.contains("\"orders\""),
        "a null did not remove its key: {body}"
    );
    assert!(
        body.contains("\"search\":null") || body.contains("\"search\": null"),
        "a nested null was stripped as if it were a removal: {body}"
    );

    // The case the merge exists for: an older client sends its whole struct,
    // which has never heard of `shell_keys`, and the newer client's key
    // survives it.
    let status = put(
        gw.port,
        &token,
        r#"{"keymap":{},"orders":{},"auto":{},"sound":"half"}"#,
    );
    assert_eq!(status, 200);
    let body = get(gw.port, &token);
    assert!(
        body.contains("shell_keys") && body.contains("\"half\""),
        "an old client's full save erased a key it did not know: {body}"
    );

    // Not an object: the client would have to guess what to do with it, and
    // a store full of bare numbers is a store nobody can migrate.
    let status = put(gw.port, &token, "[1,2,3]");
    assert_eq!(status, 400, "a JSON array was accepted as settings");

    // And not a place to park a megabyte.
    let padding = "x".repeat(32 * 1024);
    let huge = format!("{{\"note\":\"{padding}\"}}");
    let (status, _) = http(gw.port, "PUT", "/settings", Some(&token), &huge);
    assert_eq!(status, 413, "an oversized blob was accepted");
    let body = get(gw.port, &token);
    assert!(
        !body.contains("xxxx"),
        "a refused save still changed the store: {body}"
    );

    // Nor by instalments: each patch is under the limit, the merge is not,
    // and the refused one leaves the store as the first one left it.
    let half = "y".repeat(9 * 1024);
    let (status, _) = http(
        gw.port,
        "PUT",
        "/settings",
        Some(&token),
        &format!("{{\"one\":\"{half}\"}}"),
    );
    assert_eq!(status, 200, "a patch under the limit was refused");
    let status = put(gw.port, &token, &format!("{{\"two\":\"{half}\"}}"));
    assert_eq!(status, 413, "two patches merged past the limit");
    let body = get(gw.port, &token);
    assert!(
        body.contains("\"one\"") && !body.contains("\"two\"") && body.contains("shell_keys"),
        "a merge refused for size still changed the store: {body}"
    );

    // Per account.
    let other = login(gw.port, "other", "other_player");
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
