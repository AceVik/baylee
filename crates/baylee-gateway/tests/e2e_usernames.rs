//! A username (#269), end to end: its owner is shown it, and nobody else
//! ever is.
//!
//! Nobody registers or signs in with one at a gateway any more: accounts are
//! Baylee Central's (`docs/central-login.md` §"Cutover"). What an account
//! from before keeps is the name, which `/me` tells its owner and nothing
//! that shows one player to another carries.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{http, json_field, login, spawn_gateway};

#[test]
fn a_username_is_shown_to_its_owner_and_to_nobody_else() {
    let gw = spawn_gateway("usernames");
    let token = login(gw.port, "Alice.B", "Alice");

    let (status, me) = http(gw.port, "GET", "/me", Some(&token), "");
    assert_eq!(status, 200);
    assert_eq!(
        json_field(&me, "username"),
        "Alice.B",
        "its owner is shown it"
    );
    let handle = json_field(&me, "handle").to_string();
    assert!(handle.starts_with("Alice#"), "{handle}");

    // What another player may know of it is its handle, never its name.
    let (status, public) = http(
        gw.port,
        "GET",
        &format!("/players/{}", handle.replace('#', "%23")),
        Some(&token),
        "",
    );
    assert_eq!(status, 200, "{public}");
    assert!(
        !public.contains("Alice.B"),
        "a username went public: {public}"
    );
}
