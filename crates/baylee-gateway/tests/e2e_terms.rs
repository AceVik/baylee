//! The terms of use (WG-1): a gateway with a terms file asks at sign-in
//! until the account accepts the version it shows, and a gateway without
//! one answers exactly as it did before terms existed.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{http, json_field, spawn_gateway, spawn_gateway_with};

const PASSWORD: &str = "a-very-fine-password";

/// Registers, then signs in; both answers, raw.
fn register_and_login(port: u16, username: &str) -> (String, String) {
    let register = format!(
        "{{\"username\":\"{username}\",\"display_name\":\"{username}\",\"password\":\"{PASSWORD}\"}}"
    );
    let (status, registered) = http(port, "POST", "/auth/register", None, &register);
    assert_eq!(status, 200, "register: {registered}");
    (registered, login(port, username))
}

fn login(port: u16, username: &str) -> String {
    let creds = format!("{{\"username\":\"{username}\",\"password\":\"{PASSWORD}\"}}");
    let (status, body) = http(port, "POST", "/auth/login", None, &creds);
    assert_eq!(status, 200, "login: {body}");
    body
}

fn guest(port: u16) -> String {
    let (status, body) = http(port, "POST", "/auth/guest", None, "{}");
    assert_eq!(status, 200, "guest: {body}");
    body
}

fn json(body: &str) -> serde_json::Value {
    serde_json::from_str(body).unwrap_or_else(|e| panic!("{e}: {body}"))
}

#[test]
fn without_a_terms_file_nothing_changes() {
    let gw = spawn_gateway("terms-none");
    let (status, info) = http(gw.port, "GET", "/info", None, "");
    assert_eq!(status, 200);
    assert_eq!(json(&info)["terms"], serde_json::Value::Null, "{info}");
    let (status, _) = http(gw.port, "GET", "/terms", None, "");
    assert_eq!(status, 404, "a gateway without terms served some");

    // The answers carry no `terms_stale` at all, not `false`: a client from
    // before terms reads exactly what it always read.
    let (registered, logged_in) = register_and_login(gw.port, "plain");
    for answer in [registered, logged_in, guest(gw.port)] {
        assert!(json(&answer).get("terms_stale").is_none(), "{answer}");
    }
    let token = json_field(&login(gw.port, "plain"), "token").to_string();
    let (status, _) = http(
        gw.port,
        "POST",
        "/account/terms",
        Some(&token),
        r#"{"version":"x"}"#,
    );
    assert_eq!(status, 404, "accepted terms that do not exist");
}

#[test]
fn a_sign_in_asks_until_the_shown_version_is_accepted() {
    let dir = std::env::temp_dir().join(format!("baylee-terms-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("terms.md");
    std::fs::write(
        &path,
        "<!-- version: 2026-10 -->\n<!-- updated: 2026-10-07 -->\n\n# Terms\n\nBe kind.\n",
    )
    .expect("write terms");
    let gw = spawn_gateway_with(
        "terms",
        &[("BAYLEE_TERMS_PATH", path.to_string_lossy().into_owned())],
    );
    let port = gw.port;

    let (_, info) = http(port, "GET", "/info", None, "");
    assert_eq!(json(&info)["terms"], "2026-10", "{info}");
    let (status, terms) = http(port, "GET", "/terms", None, "");
    assert_eq!(status, 200);
    assert_eq!(
        json(&terms),
        serde_json::json!({
            "version": "2026-10",
            "updated": "2026-10-07",
            "markdown": "# Terms\n\nBe kind.\n",
        })
    );

    // A new account, a returning one that never accepted, a new guest: all
    // asked.
    let (registered, logged_in) = register_and_login(port, "reader");
    assert_eq!(json(&registered)["terms_stale"], true, "{registered}");
    assert_eq!(json(&logged_in)["terms_stale"], true, "{logged_in}");
    assert_eq!(json(&guest(port))["terms_stale"], true);

    let token = json_field(&logged_in, "token").to_string();
    let accept = |version: &str, token: Option<&str>| {
        http(
            port,
            "POST",
            "/account/terms",
            token,
            &format!("{{\"version\":\"{version}\"}}"),
        )
        .0
    };
    assert_eq!(accept("2026-10", None), 401, "a stranger accepted terms");
    // An older text than the one in force is not what is being agreed to.
    assert_eq!(accept("2026-09", Some(&token)), 409);
    assert_eq!(
        json(&login(port, "reader"))["terms_stale"],
        true,
        "a refused acceptance was recorded"
    );
    assert_eq!(accept("2026-10", Some(&token)), 200);
    assert_eq!(
        json(&login(port, "reader"))["terms_stale"],
        false,
        "accepted, and still asked"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
