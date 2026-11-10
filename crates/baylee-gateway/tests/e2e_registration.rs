//! End-to-end tests for who may come in new (`BAYLEE_REGISTRATION`) since
//! the closed beta's keys (#317) went with the open beta (10.11.2026):
//! `open` lets a new guest in, `off` is as it was, and `invite`, the closed
//! beta's word, stops the start and says what to set instead. A client
//! from the closed beta still sends its key; the gateway ignores it.
//!
//! Every test that serves runs a real gateway in a schema of its own. The
//! guest door takes ten requests per address in five minutes, and every
//! request here comes from `127.0.0.1`, so each test has a gateway of its
//! own for that reason too.

#![allow(clippy::missing_docs_in_private_items)]

use crate::common;

use common::{http, spawn_gateway, spawn_gateway_with};

/// The two doors say who may come in, on both routes a client asks.
fn doors(port: u16) -> (serde_json::Value, serde_json::Value) {
    let (status, info) = http(port, "GET", "/info", None, "");
    assert_eq!(status, 200, "{info}");
    let (status, config) = http(port, "GET", "/auth/config", None, "");
    assert_eq!(status, 200, "{config}");
    (
        serde_json::from_str(&info).unwrap(),
        serde_json::from_str(&config).unwrap(),
    )
}

/// `POST /auth/guest` with `body`.
fn guest(port: u16, body: &serde_json::Value) -> (u16, String) {
    http(port, "POST", "/auth/guest", None, &body.to_string())
}

/// The gateway's binary with `env` and nothing else but `PATH` and a
/// database nobody listens on: exit code and standard error.
fn start(args: &[&str], env: &[(&str, &str)]) -> (Option<i32>, String) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
        .args(args)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("DATABASE_URL", "postgres://nobody@127.0.0.1:1/nothing")
        .envs(env.iter().copied())
        .output()
        .expect("the gateway runs");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// An open gateway lets a new guest in with nothing but a name, and a
/// client from the closed beta that still sends its key, a good one, a
/// made-up one or one of the wrong type, is let in the same way: the field
/// is unknown here now and ignored.
#[test]
fn an_open_gateway_lets_a_guest_in_and_ignores_an_old_clients_key() {
    let gw = spawn_gateway_with(
        "registration_open",
        &[("BAYLEE_REGISTRATION", "open".into())],
    );
    let (info, config) = doors(gw.port);
    assert_eq!(info["registration"], "open", "{info}");
    assert_eq!(config["registration"], "open", "{config}");
    assert_eq!(
        config["registration_enabled"], false,
        "nobody signs up here"
    );
    for body in [
        serde_json::json!({ "display_name": "Visitor" }),
        serde_json::json!({ "display_name": "Visitor", "invite_key": "BAYLEE-7K3M-Q9TX-2HVD-4WNC" }),
        serde_json::json!({ "display_name": "Visitor", "invite_key": "" }),
        serde_json::json!({ "display_name": "Visitor", "invite_key": 42 }),
        serde_json::json!({ "invite_key": null }),
    ] {
        let (status, answer) = guest(gw.port, &body);
        assert_eq!(status, 200, "{body}: {answer}");
    }
    assert_eq!(gw.scalar("SELECT count(*) FROM account WHERE guest"), 5);
}

/// Unset is open, as it always was.
#[test]
fn an_unset_gateway_is_open() {
    let gw = spawn_gateway("registration_unset");
    let (info, config) = doors(gw.port);
    assert_eq!(info["registration"], "open", "{info}");
    assert_eq!(config["registration"], "open", "{config}");
}

/// A gateway with registration off is as it was: it says so, and its guest
/// door is the auth mode's to shut, not this switch's.
#[test]
fn a_gateway_with_registration_off_is_as_it_was() {
    let gw = spawn_gateway_with("registration_off", &[("BAYLEE_REGISTRATION", "off".into())]);
    let (info, config) = doors(gw.port);
    assert_eq!(info["registration"], "off", "{info}");
    assert_eq!(config["registration"], "off", "{config}");
    assert_eq!(config["registration_enabled"], false);
    let (status, answer) = guest(gw.port, &serde_json::json!({ "display_name": "Visitor" }));
    assert_eq!(status, 200, "{answer}");
}

/// `BAYLEE_REGISTRATION=invite` stops the start before the database, and
/// names the two settings left: neither is guessed for the operator.
#[test]
fn the_closed_betas_setting_stops_the_start_and_names_the_fix() {
    let (code, stderr) = start(&[], &[("BAYLEE_REGISTRATION", "invite")]);
    assert_ne!(code, Some(0), "{stderr}");
    assert!(stderr.contains("BAYLEE_REGISTRATION"), "{stderr}");
    // In its own words, so not the database's: nothing listens there.
    assert!(stderr.contains("BAYLEE_REGISTRATION=open"), "{stderr}");
}

/// `baylee-gateway invite …`, the command that made the keys, says they are
/// gone and stops, before it reads a setting or opens the database: it
/// never starts a gateway in their place, whatever the shell held.
#[test]
fn the_key_command_is_gone_and_starts_nothing() {
    for args in [
        &["invite"][..],
        &["invite", "list"][..],
        &["invite", "create", "--note", "Max"][..],
        &["invite", "revoke", "0190a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b"][..],
    ] {
        let (code, stderr) = start(args, &[("PORT", "0"), ("BAYLEE_REGISTRATION", "open")]);
        assert_eq!(code, Some(2), "{args:?}: {stderr}");
        assert!(
            stderr.contains("the closed beta's keys are gone"),
            "{args:?}: {stderr}"
        );
        assert!(!stderr.contains("panicked"), "{args:?}: {stderr}");
    }
}
