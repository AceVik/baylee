//! Signing in with a username (#269), end to end.
//!
//! A player registers a username and signs in with it; nobody else is ever
//! shown it. An account that registered with an address before usernames
//! was given one from it, and signs in by either until the end of 2026,
//! being told its username when it uses the address. Every way of naming
//! one account spends the same eight tries.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{Gateway, http, json_field, spawn_gateway};

const PASSWORD: &str = "a-very-fine-password";

fn register(port: u16, username: &str, name: &str, password: &str) -> (u16, String) {
    let body = format!(
        "{{\"username\":\"{username}\",\"display_name\":\"{name}\",\"password\":\"{password}\"}}"
    );
    http(port, "POST", "/auth/register", None, &body)
}

fn sign_in(port: u16, who: &str, password: &str) -> (u16, String) {
    let body = format!("{{\"username\":\"{who}\",\"password\":\"{password}\"}}");
    http(port, "POST", "/auth/login", None, &body)
}

/// An account as registration made them before usernames: with an address.
fn give_an_address(gw: &Gateway, username: &str, address: &str) {
    gw.sql(&format!(
        "UPDATE account SET email = '{address}' WHERE username_key = '{username}'"
    ));
}

#[test]
fn a_player_signs_in_with_their_username_in_any_case_and_only_they_see_it() {
    let gw = spawn_gateway("usernames");
    let (status, body) = register(gw.port, "Alice.B", "Alice", PASSWORD);
    assert_eq!((status, body.as_str()), (200, r#"{"ok":true}"#));

    for typed in ["Alice.B", "alice.b", "ＡＬＩＣＥ.b"] {
        let (status, body) = sign_in(gw.port, typed, PASSWORD);
        assert_eq!(status, 200, "{typed}: {body}");
        assert_eq!(
            json_field(&body, "username"),
            "Alice.B",
            "as it was typed at registration"
        );
    }
    // A client from before usernames calls the field `email`.
    let (status, body) = http(
        gw.port,
        "POST",
        "/auth/login",
        None,
        &format!(r#"{{"email":"alice.b","password":"{PASSWORD}"}}"#),
    );
    assert_eq!(status, 200, "the old field name: {body}");
    let token = json_field(&body, "token").to_string();

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

/// A username is findable by design, so a taken one is said to be taken; a
/// display name is not unique at all.
#[test]
fn a_taken_username_is_refused_openly_whatever_its_case() {
    let gw = spawn_gateway("usernames-taken");
    let (status, body) = register(gw.port, "twice", "Twice", PASSWORD);
    assert_eq!(status, 200, "{body}");
    for again in ["twice", "TWICE", "ｔｗｉｃｅ"] {
        let (status, body) = register(gw.port, again, "Somebody", "another-fine-password");
        assert_eq!(status, 409, "{again}: {body}");
        assert!(body.contains("that username is taken"), "{body}");
    }
    // The first account is untouched: its own password, not the second's.
    assert_eq!(sign_in(gw.port, "twice", PASSWORD).0, 200);
    assert_eq!(sign_in(gw.port, "twice", "another-fine-password").0, 401);

    let (status, body) = register(gw.port, "other", "Twice", PASSWORD);
    assert_eq!(status, 200, "a display name shared with somebody: {body}");
}

#[test]
fn a_name_the_rule_refuses_and_a_password_that_is_a_name_are_refused() {
    let gw = spawn_gateway("usernames-rule");
    for bad in ["ab", "a..b", ".alice", "ali\\u200bce", "alice@home"] {
        let (status, body) = register(gw.port, bad, "Alice", PASSWORD);
        assert_eq!(status, 400, "{bad}: {body}");
        assert!(body.contains("invalid username"), "{bad}: {body}");
    }
    // A client from before usernames sends an address and no name.
    let (status, body) = http(
        gw.port,
        "POST",
        "/auth/register",
        None,
        &format!(r#"{{"email":"a@example.com","display_name":"Alice","password":"{PASSWORD}"}}"#),
    );
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("invalid username"), "{body}");

    // Neither name, in any case, is a password.
    let (status, body) = register(gw.port, "wonderland", "Alice", "WonderLand");
    assert_eq!(status, 400, "the username as the password: {body}");
    let (status, body) = register(gw.port, "wonderland", "Rabbit_hole", "rabbit_HOLE");
    assert_eq!(status, 400, "the display name as the password: {body}");
    let (status, body) = register(gw.port, "wonderland", "Rabbit_hole", PASSWORD);
    assert_eq!(status, 200, "{body}");
}

/// An account that signed in with an address before usernames signs in with
/// it until the end of 2026, and is told the name it was given.
#[test]
fn an_address_still_signs_in_and_is_told_the_username() {
    let gw = spawn_gateway("usernames-legacy");
    let (status, body) = register(gw.port, "legacy", "Legacy", PASSWORD);
    assert_eq!(status, 200, "{body}");
    give_an_address(&gw, "legacy", "Legacy@Example.com");

    let (status, body) = sign_in(gw.port, "legacy@example.COM", PASSWORD);
    assert_eq!(status, 200, "{body}");
    assert_eq!(json_field(&body, "username"), "legacy");
    assert_eq!(sign_in(gw.port, "legacy@example.com", "not-it").0, 401);
    assert_eq!(sign_in(gw.port, "nobody@example.com", PASSWORD).0, 401);
}

/// Sign-in attempts are counted against the **account**, however it was
/// named, and not against the machine.
///
/// Keyed per IP, one development box is one bucket: the owner's typing and
/// every scripted call shared ten attempts, and the owner was the one locked
/// out. Keyed per spelling, an account with a username and an address would
/// have had two budgets, sixteen guesses where it should have eight. And a
/// wrong password at one account must not cost another anything.
#[test]
fn eight_tries_are_eight_tries_at_one_account_however_it_is_named() {
    let gw = spawn_gateway("attempts");
    let port = gw.port;
    assert_eq!(register(port, "eight", "Eight", PASSWORD).0, 200);
    give_an_address(&gw, "eight", "eight@example.com");
    assert_eq!(register(port, "other", "Other", PASSWORD).0, 200);

    let wrong = |who: &str| sign_in(port, who, "not-it").0;

    // Five by the name and three by the address are the eight answered on
    // their merits; after that neither way in is answered at all.
    for i in 1..=5 {
        assert_eq!(wrong("eight"), 401, "try {i} by name");
    }
    for i in 6..=8 {
        assert_eq!(wrong("eight@example.com"), 401, "try {i} by address");
    }
    assert_eq!(wrong("Eight@Example.com"), 429, "the ninth, by address");
    assert_eq!(wrong("EIGHT"), 429, "the tenth, by name");
    assert_eq!(
        sign_in(port, "eight", PASSWORD).0,
        429,
        "not even the right password, until the window passes"
    );

    // And the machine is not what was counted.
    assert_eq!(
        wrong("other"),
        401,
        "a neighbour's typing costs this account nothing"
    );
    assert_eq!(sign_in(port, "other", PASSWORD).0, 200);

    // A name that is nobody's is counted too, under what was typed.
    for i in 1..=8 {
        assert_eq!(wrong("nobody-here"), 401, "try {i} at nobody");
    }
    assert_eq!(
        wrong("Nobody-Here"),
        429,
        "guessing at a name is bounded as well"
    );
}

/// Getting in is what the window was counting towards, so it hands the tries
/// back: a player who mistypes seven times, signs in, then signs in again
/// has not spent anything.
#[test]
fn signing_in_clears_what_the_typos_spent() {
    let gw = spawn_gateway("attempts-cleared");
    let port = gw.port;
    assert_eq!(register(port, "clear", "Clear", PASSWORD).0, 200);
    for _ in 0..7 {
        assert_eq!(sign_in(port, "clear", "not-it").0, 401);
    }
    let (status, body) = sign_in(port, "clear", PASSWORD);
    assert_eq!(status, 200, "the eighth try is the right one: {body}");
    // Without the clearing this would be the ninth attempt and refused.
    let (status, body) = sign_in(port, "clear", PASSWORD);
    assert_eq!(status, 200, "and signing in again is not rationed: {body}");
}
