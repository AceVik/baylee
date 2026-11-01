//! End-to-end tests for the closed beta (#317): a gateway with
//! `BAYLEE_REGISTRATION=invite` makes a new guest only for somebody with a
//! key its operator made with `baylee-gateway invite`. Nobody registers here
//! any more (`docs/central-login.md` §"Cutover"), so the guest door is the
//! one a key opens.
//!
//! Every test here runs a real gateway in a schema of its own and makes its
//! keys with the real command against that schema, so a key a test comes in
//! with is one the operator's command printed. Each test has a gateway of
//! its own for a second reason too: the guest door takes ten requests per
//! address in five minutes, and every request here comes from `127.0.0.1`.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{Gateway, http, json_field, spawn_gateway, spawn_gateway_with};

/// What a key that admits nobody is answered, whatever the reason.
const INVALID: &str = r#"{"error":"this closed beta key is not valid"}"#;

/// The refusal of a request that brought no key to a closed beta.
const NEEDED: &str = "this gateway is a closed beta: a new account or guest needs a closed \
                      beta key. If there is no field for one, update Baylee";

/// A gateway that wants a key.
fn closed_beta(label: &str) -> Gateway {
    spawn_gateway_with(label, &[("BAYLEE_REGISTRATION", "invite".into())])
}

/// `baylee-gateway invite <args>` against this gateway's schema: exit code,
/// standard output, standard error. Run with the gateway's own logging
/// switched up, so that a log line on standard output would be seen here.
fn invite(gw: &Gateway, args: &[&str]) -> (i32, String, String) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
        .arg("invite")
        .args(args)
        .env("DATABASE_URL", gw.database_url())
        .env("RUST_LOG", "debug")
        .output()
        .expect("the command runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Keys made by the command, as it printed them.
fn keys(gw: &Gateway, args: &[&str]) -> Vec<String> {
    let mut all = vec!["create"];
    all.extend_from_slice(args);
    let (code, out, err) = invite(gw, &all);
    assert_eq!(code, 0, "create: {out}{err}");
    out.lines().map(str::to_owned).collect()
}

/// One key, one use.
fn a_key(gw: &Gateway) -> String {
    let made = keys(gw, &[]);
    assert_eq!(made.len(), 1, "{made:?}");
    made.into_iter().next().unwrap()
}

/// `POST /auth/guest`, with `key` if there is one.
fn guest(port: u16, key: Option<&str>) -> (u16, String) {
    let body = match key {
        Some(key) => serde_json::json!({ "display_name": "Visitor", "invite_key": key }),
        None => serde_json::json!({ "display_name": "Visitor" }),
    };
    http(port, "POST", "/auth/guest", None, &body.to_string())
}

/// A key of the right shape that nobody made.
const NEVER_MADE: &str = "BAYLEE-0000-0000-0000-0000";

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

/// The acceptance of #317 at the one door a key still opens: no key, a
/// wrong one, a used one, a revoked one and an expired one are refused, the
/// last four in one body; a key the command printed lets one new guest in,
/// however it is typed. A guest already made comes back with its session
/// and no key.
///
/// Nine requests, one short of what the guest door takes from one address.
#[test]
fn a_closed_beta_takes_a_new_guest_only_with_a_key_and_each_key_once() {
    let gw = closed_beta("invite_guests");
    let port = gw.port;

    let (info, config) = doors(port);
    assert_eq!(info["registration"], "invite", "{info}");
    assert_eq!(info["guests"], true, "{info}");
    assert_eq!(config["registration"], "invite", "{config}");
    assert_eq!(
        config["registration_enabled"], false,
        "nobody signs up at a gateway: {config}"
    );

    let (status, body) = guest(port, None);
    assert_eq!((status, json_field(&body, "error")), (403, NEEDED));
    let (status, body) = guest(port, Some("   "));
    assert_eq!(
        (status, json_field(&body, "error")),
        (403, NEEDED),
        "blank is none"
    );

    let key = a_key(&gw);
    let (status, body) = guest(port, Some(&key));
    assert_eq!(status, 200, "{body}");
    let token = json_field(&body, "token").to_string();
    assert_eq!(
        gw.scalar("SELECT count(*) FROM account WHERE guest AND invite_id IS NOT NULL"),
        1,
        "the guest names its key"
    );

    // A second use of a one-use key, and every other refusal, read alike.
    let revoked = a_key(&gw);
    let id = gw.text("SELECT id::text FROM invite ORDER BY created_at DESC, id DESC LIMIT 1");
    let (code, out, err) = invite(&gw, &["revoke", &id]);
    assert_eq!(code, 0, "{out}{err}");
    let expired = a_key(&gw);
    gw.sql(
        "UPDATE invite SET expires_at = now() - interval '1 second' \
         WHERE id = (SELECT id FROM invite ORDER BY created_at DESC, id DESC LIMIT 1)",
    );
    for (what, typed) in [
        ("used", key.as_str()),
        ("never made", NEVER_MADE),
        ("not a key", "hello"),
        ("revoked", revoked.as_str()),
        ("expired", expired.as_str()),
    ] {
        let (status, body) = guest(port, Some(typed));
        assert_eq!((status, body.as_str()), (403, INVALID), "{what}");
    }

    // Typed as a person types it off a message: lower case, spaces for
    // dashes, O for 0 and L for 1.
    let sloppy = a_key(&gw).to_lowercase().replace('-', " ");
    let sloppy = sloppy.replace('0', "o").replace('1', "l");
    let (status, body) = guest(port, Some(&sloppy));
    assert_eq!(status, 200, "{sloppy}: {body}");
    assert_eq!(
        gw.scalar("SELECT count(*) FROM account"),
        2,
        "two guests, and none of the refused"
    );

    // The guest this device kept comes back with its session alone.
    let (status, me) = http(port, "GET", "/me", Some(&token), "");
    assert_eq!(status, 200, "{me}");
    assert!(me.contains("\"guest\":true"), "{me}");
    let (status, decks) = http(port, "GET", "/decks", Some(&token), "");
    assert_eq!(status, 200, "{decks}");
}

/// A mode without guests (`BAYLEE_AUTH_MODE=central`) shuts the guest
/// door whatever key is brought.
#[test]
fn no_guests_is_no_guests_even_with_a_key() {
    let gw = spawn_gateway_with(
        "invite_no_guests",
        &[
            ("BAYLEE_REGISTRATION", "invite".into()),
            ("BAYLEE_AUTH_MODE", "central".into()),
        ],
    );
    let (info, _) = doors(gw.port);
    assert_eq!(info["guests"], false, "{info}");
    let key = a_key(&gw);
    let (status, body) = guest(gw.port, Some(&key));
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("takes no guests"), "{body}");
    assert_eq!(
        gw.scalar("SELECT uses_left::bigint FROM invite"),
        1,
        "the key was not spent on the refused guest"
    );
}

/// Guessing at keys is bounded: eight tries from one address, and the ninth
/// is refused before it is read, even when it is a good key. The good key is
/// not spent by the refusal. Nine requests, inside the ten the guest door
/// takes from one address, so the refusal is the keys' own.
#[test]
fn guessing_at_keys_runs_into_the_limiter() {
    let gw = closed_beta("invite_limit");
    let port = gw.port;
    let key = a_key(&gw);
    for n in 0..8 {
        let guess = if n % 2 == 0 {
            format!("BAYLEE-0000-0000-0000-000{n}")
        } else {
            "not a key at all".to_owned()
        };
        let (status, body) = guest(port, Some(&guess));
        assert_eq!((status, body.as_str()), (403, INVALID), "try {n}");
    }
    let (status, body) = guest(port, Some(&key));
    assert_eq!(status, 429, "the ninth try: {body}");
    assert_eq!(
        gw.scalar("SELECT uses_left::bigint FROM invite"),
        1,
        "the refused try spent nothing"
    );
}

/// The gateway logs no key, even at debug: not as printed, not as typed,
/// not as it is read.
#[test]
fn no_key_reaches_the_log() {
    let gw = spawn_gateway_with(
        "invite_logs",
        &[
            ("BAYLEE_REGISTRATION", "invite".into()),
            ("RUST_LOG", "debug".into()),
        ],
    );
    let port = gw.port;
    let made = keys(&gw, &["--count", "2"]);
    let typed = made[1].to_lowercase().replace('-', " ");
    assert_eq!(guest(port, Some(&made[0])).0, 200);
    assert_eq!(guest(port, Some(&typed)).0, 200);
    assert_eq!(guest(port, Some(&made[0])).0, 403);
    assert_eq!(guest(port, Some("BAYLEE-ZZZZ-ZZZZ-ZZZZ-ZZZY")).0, 403);

    let logs = gw.logs();
    assert!(
        logs.contains("DEBUG") || logs.contains("INFO"),
        "the gateway logged at debug, or this proves nothing: {logs}"
    );
    for key in &made {
        let canonical: String = key["BAYLEE-".len()..].replace('-', "");
        for form in [
            key.as_str(),
            canonical.as_str(),
            &key.to_lowercase(),
            &canonical.to_lowercase(),
        ] {
            assert!(!logs.contains(form), "{form} is in the log");
        }
        // Any four characters of it in a row, in case it was cut or grouped.
        for group in key.split('-').skip(1) {
            assert!(
                !logs.contains(&format!("-{group}")),
                "{group} is in the log"
            );
        }
    }
    assert!(!logs.contains(&typed));
    assert!(!logs.contains("ZZZZ-ZZZY"));
}

/// The operator's command: `create` prints the keys and nothing else on
/// standard output, `list` shows everything but a key, and `revoke` shuts
/// one. It needs the database and no running gateway.
#[test]
fn the_command_makes_lists_and_revokes_keys() {
    let gw = closed_beta("invite_cli");

    let made = keys(
        &gw,
        &[
            "--count",
            "3",
            "--uses",
            "2",
            "--expires",
            "30d",
            "--note",
            "for Max",
        ],
    );
    assert_eq!(made.len(), 3, "{made:?}");
    for key in &made {
        assert!(key.starts_with("BAYLEE-") && key.len() == 26, "{key}");
    }
    let (code, out, err) = invite(&gw, &["create", "--note", "Moritz"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out.lines().count(), 1, "one key and nothing else: {out}");
    assert!(err.contains("for Moritz"), "{err}");

    let (code, listed, err) = invite(&gw, &["list"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        listed.lines().count(),
        5,
        "a header and four keys: {listed}"
    );
    assert_eq!(listed.matches("for Max").count(), 3, "{listed}");
    assert!(listed.contains("Moritz"), "{listed}");
    for key in made
        .iter()
        .chain(out.lines().map(str::to_owned).collect::<Vec<_>>().iter())
    {
        let canonical = key["BAYLEE-".len()..].replace('-', "");
        assert!(!listed.contains(key.as_str()), "list printed {key}");
        assert!(!listed.contains(&canonical), "list printed {canonical}");
        assert!(!err.contains(&canonical));
    }
    assert!(!listed.contains("BAYLEE-"), "{listed}");

    // A key the command printed lets a guest in, twice for two uses.
    assert_eq!(guest(gw.port, Some(&made[0])).0, 200);
    assert_eq!(guest(gw.port, Some(&made[0])).0, 200);
    let (_, listed, _) = invite(&gw, &["list"]);
    let row = listed
        .lines()
        .find(|l| l.contains("for Max") && l.split_whitespace().any(|w| w == "2"))
        .is_some();
    assert!(row, "an unspent key still shows two uses: {listed}");

    // Revoke the second, found by the SHA-256 of its sixteen characters,
    // which is all the table holds of it. It admits nobody from then on, the
    // third still does, and revoking it again says so.
    let canonical = made[1]["BAYLEE-".len()..].replace('-', "");
    let id = gw.text(&format!(
        "SELECT id::text FROM invite WHERE key_hash = sha256(convert_to('{canonical}', 'UTF8'))"
    ));
    let (code, out, err) = invite(&gw, &["revoke", &id]);
    assert_eq!(
        (code, out.trim()),
        (0, format!("revoked {id}").as_str()),
        "{err}"
    );
    let (code, _, err) = invite(&gw, &["revoke", &id]);
    assert_eq!(code, 1, "revoked twice: {err}");
    let (status, body) = guest(gw.port, Some(&made[1]));
    assert_eq!((status, body.as_str()), (403, INVALID));
    assert_eq!(guest(gw.port, Some(&made[2])).0, 200);
    let (_, listed, _) = invite(&gw, &["list"]);
    assert_eq!(
        listed.lines().filter(|l| l.starts_with(&id)).count(),
        1,
        "{listed}"
    );
    assert!(
        !listed
            .lines()
            .find(|l| l.starts_with(&id))
            .unwrap()
            .contains(" - "),
        "a revoked key shows when: {listed}"
    );

    // Refusals: a bad option exits 2, with the usage.
    let (code, _, err) = invite(&gw, &["create", "--uses", "0"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("usage"), "{err}");
    let (code, _, err) = invite(&gw, &["create", "--note", &"x".repeat(101)]);
    assert_eq!(code, 2, "{err}");
}

/// An open gateway is as it was: no key asked, and one sent is ignored.
#[test]
fn an_open_gateway_asks_no_key() {
    let gw = spawn_gateway("invite_open");
    let (info, config) = doors(gw.port);
    assert_eq!(info["registration"], "open", "{info}");
    assert_eq!(config["registration"], "open", "{config}");
    assert_eq!(
        config["registration_enabled"], false,
        "nobody signs up here"
    );
    assert_eq!(guest(gw.port, None).0, 200);
    assert_eq!(guest(gw.port, Some("whatever")).0, 200);
    assert_eq!(
        gw.scalar("SELECT count(*) FROM account WHERE invite_id IS NOT NULL"),
        0
    );
}

/// A gateway with registration off is as it was, and its guests need no
/// key: only `invite` closes the guest door.
#[test]
fn a_gateway_with_registration_off_is_as_it_was() {
    let gw = spawn_gateway_with("invite_off", &[("BAYLEE_REGISTRATION", "off".into())]);
    let (info, config) = doors(gw.port);
    assert_eq!(info["registration"], "off", "{info}");
    assert_eq!(config["registration"], "off", "{config}");
    assert_eq!(config["registration_enabled"], false);
    assert_eq!(guest(gw.port, None).0, 200);
}
