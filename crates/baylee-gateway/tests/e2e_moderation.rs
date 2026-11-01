//! End-to-end tests for acting on an account or a game from the admin
//! console (beta.8, `docs/protocol.md` §"Acting on an account" and §"Acting
//! on a game"): a real gateway with `BAYLEE_ADMIN_TOKEN`, asked the way the
//! feedback service asks it, and the public doors those acts close.
//!
//! What they hold: a ban signs a registered account's sessions out and
//! refuses any session of it with its reason, a guest's included; lifting it
//! and deleting an account work; the console rows say who is banned and by
//! whom; ending a game tells every seat why and ends it; the password reset
//! went with the gateway's own passwords (`docs/central-login.md`
//! §"Cutover"); and no audit line carries a token or a reason.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::v1;
use common::{Gateway, attach_agent, http, json_field, login, spawn_gateway_with};

/// The token every console here is started with.
const TOKEN: &str = "e2e-admin-token-0123456789abcdef0123456789";

/// The admin the feedback service would name.
const ADMIN: &str = "viktor";

/// A gateway and its console's port.
struct Console {
    gw: Gateway,
    port: u16,
}

fn console(label: &str) -> Console {
    let port_file = std::env::temp_dir().join(format!(
        "baylee-moderation-{label}-{}-{}.port",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&port_file);
    let env = vec![
        ("BAYLEE_ADMIN_TOKEN", TOKEN.to_owned()),
        ("BAYLEE_ADMIN_BIND", "127.0.0.1:0".to_owned()),
        (
            "BAYLEE_ADMIN_PORT_FILE",
            port_file.to_string_lossy().into_owned(),
        ),
        ("RUST_LOG", "baylee_gateway::audit=info,warn".to_owned()),
    ];
    let gw = spawn_gateway_with(label, &env);
    let port = std::fs::read_to_string(&port_file)
        .expect("the console's port file")
        .trim()
        .parse()
        .expect("a port");
    let _ = std::fs::remove_file(&port_file);
    Console { gw, port }
}

/// One console request as the feedback service sends it: status and body.
fn admin(c: &Console, method: &str, path: &str, body: &str) -> (u16, String) {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", c.port)).expect("connect");
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {TOKEN}\r\nX-Baylee-Admin: {ADMIN}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        c.port,
        body.len()
    );
    stream.write_all(request.as_bytes()).expect("write");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read");
    let status = raw
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("a status");
    let body = raw.split_once("\r\n\r\n").map_or("", |(_, b)| b).to_owned();
    (status, body)
}

/// A log without its colours (`ESC [ … m`).
fn plain(logs: &str) -> String {
    let mut out = String::with_capacity(logs.len());
    let mut chars = logs.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn json(body: &str) -> serde_json::Value {
    serde_json::from_str(body).unwrap_or_else(|e| panic!("{e}: {body}"))
}

/// An account's id, as the console lists it.
fn id_of(c: &Console, username: &str) -> String {
    let (status, body) = admin(c, "GET", &format!("/admin/accounts?q={username}"), "");
    assert_eq!(status, 200, "{body}");
    let rows = json(&body);
    rows["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["username"] == username)
        .unwrap_or_else(|| panic!("{username} is not listed: {rows}"))["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// A guest: its id and its token.
fn a_guest(c: &Console) -> (String, String) {
    let (status, answer) = http(
        c.gw.port,
        "POST",
        "/auth/guest",
        None,
        r#"{"display_name":"Visitor"}"#,
    );
    assert_eq!(status, 200, "guest: {answer}");
    let token = json_field(&answer, "token").to_owned();
    let (_, body) = admin(c, "GET", "/admin/accounts?kind=guest", "");
    let id = json(&body)["accounts"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    (id, token)
}

/// A new session for an account `common::login` made: its bearer token.
///
/// Written into the table as `common::login` writes the first, because no
/// route here makes one: Baylee Central's sign-in does, and refuses a banned
/// account. So this is the session that would slip in beside a ban, and
/// what a ban does to a session is seen on it. An account that is not there
/// fails the insert rather than the test's next assertion.
fn another_session(c: &Console, username: &str) -> String {
    use sha2::Digest as _;
    use std::fmt::Write as _;
    let hex = |bytes: &[u8]| {
        bytes.iter().fold(String::new(), |mut out, b| {
            write!(out, "{b:02x}").expect("writing to a String");
            out
        })
    };
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS randomness");
    let token = hex(&bytes);
    let digest = hex(&sha2::Sha256::digest(token.as_bytes()));
    c.gw.sql(&format!(
        "INSERT INTO session_token (token_hash, account_id, expires_at)
         VALUES (decode('{digest}', 'hex'),
                 (SELECT id FROM account WHERE username_key = '{}'),
                 clock_timestamp() + interval '12 hours')",
        username.to_ascii_lowercase()
    ));
    token
}

/// `GET /me` with `token`: status and body.
fn me(c: &Console, token: &str) -> (u16, serde_json::Value) {
    let (status, body) = http(c.gw.port, "GET", "/me", Some(token), "");
    (status, json(&body))
}

fn me_status(c: &Console, token: &str) -> u16 {
    me(c, token).0
}

/// The reset went with the gateway's own passwords: on the console it is no
/// route, for an account that exists as for one that does not, and the
/// public port never had it.
#[test]
fn a_password_reset_is_no_route() {
    let c = console("mod_reset_gone");
    let _ = login(c.gw.port, "alice", "Alice");
    let id = id_of(&c, "alice");
    for account in [id.as_str(), "0199aaaa-0000-7000-8000-000000000001"] {
        let path = format!("/admin/accounts/{account}/password");
        let (status, body) = admin(&c, "POST", &path, "");
        assert_eq!(status, 404, "{path}: {body}");
        assert_eq!(json(&body)["error"], "no such route", "{body}");
        let (status, _) = http(c.gw.port, "POST", &path, None, "");
        assert_eq!(status, 404, "the public port: {path}");
    }
    let (_, body) = admin(&c, "GET", &format!("/admin/accounts/{id}"), "");
    assert_eq!(json(&body)["reset_pending"], false, "{body}");
}

#[test]
fn a_banned_account_is_signed_out_and_any_session_of_it_is_told_why() {
    let c = console("mod_ban");
    let session = login(c.gw.port, "carl", "Carl");
    let id = id_of(&c, "carl");

    let order = r#"{"reason":"cheating at the table","until":"2999-01-01T00:00:00Z"}"#;
    let (status, body) = admin(&c, "PUT", &format!("/admin/accounts/{id}/ban"), order);
    assert_eq!(status, 200, "{body}");
    let ban = &json(&body)["ban"];
    assert_eq!(ban["reason"], "cheating at the table", "{body}");
    assert_eq!(ban["until"], "2999-01-01T00:00:00Z", "{body}");

    assert_eq!(me_status(&c, &session), 401, "its sessions are signed out");
    // A session that slipped in beside the ban is refused with its reason.
    let slipped = another_session(&c, "carl");
    let (status, answer) = me(&c, &slipped);
    assert_eq!(status, 403, "{answer}");
    assert_eq!(
        answer["error"],
        "This account is banned until 2999-01-01T00:00:00Z. Reason: cheating at the table",
        "{answer}"
    );
    assert_eq!(answer["ban"]["reason"], "cheating at the table", "{answer}");
    assert_eq!(answer["ban"]["until"], "2999-01-01T00:00:00Z", "{answer}");

    // The console shows who, why and until when.
    let (_, body) = admin(&c, "GET", &format!("/admin/accounts/{id}"), "");
    let detail = json(&body);
    let shown = &detail["ban"];
    assert_eq!(shown["reason"], "cheating at the table", "{detail}");
    assert_eq!(shown["by"], ADMIN, "{detail}");
    assert_eq!(shown["until"], "2999-01-01T00:00:00Z", "{detail}");
    assert!(shown["at"].is_string(), "{detail}");
    let (_, body) = admin(&c, "GET", "/admin/accounts?q=carl", "");
    assert_eq!(
        json(&body)["accounts"][0]["ban"]["reason"],
        "cheating at the table",
        "{body}"
    );

    // Lifted: the same session is the account again.
    let (status, _) = admin(&c, "DELETE", &format!("/admin/accounts/{id}/ban"), "");
    assert_eq!(status, 204);
    let (status, answer) = me(&c, &slipped);
    assert_eq!(status, 200, "{answer}");
    let (_, body) = admin(&c, "GET", &format!("/admin/accounts/{id}"), "");
    assert!(json(&body)["ban"].is_null(), "{body}");
    let (status, body) = admin(&c, "DELETE", &format!("/admin/accounts/{id}/ban"), "");
    assert_eq!(status, 404, "nothing to lift: {body}");
}

#[test]
fn a_permanent_ban_has_no_end_and_a_lapsed_one_is_none() {
    let c = console("mod_ban_end");
    let _ = login(c.gw.port, "dora", "Dora");
    let id = id_of(&c, "dora");
    let (status, body) = admin(
        &c,
        "PUT",
        &format!("/admin/accounts/{id}/ban"),
        r#"{"reason":"for good"}"#,
    );
    assert_eq!(status, 200, "{body}");
    assert!(json(&body)["ban"]["until"].is_null(), "{body}");
    let session = another_session(&c, "dora");
    let (status, answer) = me(&c, &session);
    assert_eq!(status, 403, "{answer}");
    assert_eq!(
        answer["error"], "This account is banned. Reason: for good",
        "{answer}"
    );
    assert!(answer["ban"]["until"].is_null(), "{answer}");

    // Replaced by a ban that ended an hour ago (set in the schema: the
    // route refuses a past end): no ban any more, here or in the console.
    c.gw.sql(&format!(
        "UPDATE account SET banned_until = now() - interval '1 hour' WHERE id = '{id}'"
    ));
    let (status, answer) = me(&c, &session);
    assert_eq!(status, 200, "a lapsed ban is no ban: {answer}");
    let (_, body) = admin(&c, "GET", &format!("/admin/accounts/{id}"), "");
    assert!(json(&body)["ban"].is_null(), "{body}");
}

#[test]
fn a_ban_order_that_is_not_one_is_refused() {
    let c = console("mod_ban_refused");
    let session = login(c.gw.port, "emil", "Emil");
    let id = id_of(&c, "emil");
    let long = format!(r#"{{"reason":"{}"}}"#, "x".repeat(501));
    let on_the_limit = format!(r#"{{"reason":"{}"}}"#, "y".repeat(500));
    for (order, why) in [
        (r#"{"reason":""}"#.to_owned(), "empty"),
        (r#"{"reason":"   "}"#.to_owned(), "blank"),
        (long, "too long"),
        (r#"{"reason":"two\nlines"}"#.to_owned(), "a newline"),
        (
            r#"{"reason":"x","until":"2020-01-01T00:00:00Z"}"#.to_owned(),
            "in the past",
        ),
        (
            r#"{"reason":"x","until":"next week"}"#.to_owned(),
            "not a moment",
        ),
        (
            r#"{"until":"2999-01-01T00:00:00Z"}"#.to_owned(),
            "no reason",
        ),
        (
            r#"{"reason":"x","note":"y"}"#.to_owned(),
            "an unknown field",
        ),
        ("not json".to_owned(), "not json"),
    ] {
        let (status, body) = admin(&c, "PUT", &format!("/admin/accounts/{id}/ban"), &order);
        assert_eq!(status, 400, "{why}: {body}");
    }
    assert_eq!(me_status(&c, &session), 200, "nothing was banned");
    let (status, body) = admin(
        &c,
        "PUT",
        &format!("/admin/accounts/{id}/ban"),
        &on_the_limit,
    );
    assert_eq!(status, 200, "five hundred characters are allowed: {body}");
    let (status, body) = admin(
        &c,
        "PUT",
        "/admin/accounts/0199aaaa-0000-7000-8000-000000000001/ban",
        r#"{"reason":"x"}"#,
    );
    assert_eq!(status, 404, "{body}");
}

#[test]
fn a_banned_guest_keeps_its_session_and_is_refused_with_it() {
    let c = console("mod_ban_guest");
    let (id, token) = a_guest(&c);
    assert_eq!(me_status(&c, &token), 200);
    let (status, body) = admin(
        &c,
        "PUT",
        &format!("/admin/accounts/{id}/ban"),
        r#"{"reason":"spam in names"}"#,
    );
    assert_eq!(status, 200, "{body}");
    let (_, body) = admin(&c, "GET", &format!("/admin/accounts/{id}"), "");
    assert_eq!(json(&body)["ban"]["reason"], "spam in names", "{body}");
    for (method, path, request) in [
        ("GET", "/me", ""),
        ("GET", "/decks", ""),
        ("POST", "/ws-ticket", r#"{"socket":"lobby"}"#),
    ] {
        let (status, body) = http(c.gw.port, method, path, Some(&token), request);
        assert_eq!(status, 403, "{method} {path}: {body}");
        assert_eq!(json(&body)["ban"]["reason"], "spam in names", "{body}");
        assert!(json(&body)["ban"]["until"].is_null(), "{body}");
    }
    // Lifted, the same session is the guest again.
    let (status, _) = admin(&c, "DELETE", &format!("/admin/accounts/{id}/ban"), "");
    assert_eq!(status, 204);
    assert_eq!(me_status(&c, &token), 200);
}

#[test]
fn a_deleted_account_is_gone_and_a_second_delete_finds_nobody() {
    let c = console("mod_delete");
    let session = login(c.gw.port, "fred", "Fred");
    let id = id_of(&c, "fred");
    let (status, body) = admin(&c, "DELETE", &format!("/admin/accounts/{id}"), "");
    assert_eq!(status, 204, "{body}");
    assert_eq!(me_status(&c, &session), 401);
    let (status, _) = admin(&c, "GET", &format!("/admin/accounts/{id}"), "");
    assert_eq!(status, 404);
    let (status, _) = admin(&c, "DELETE", &format!("/admin/accounts/{id}"), "");
    assert_eq!(status, 404);
    let (status, _) = admin(&c, "DELETE", "/admin/accounts/nobody", "");
    assert_eq!(status, 400);
    assert_eq!(c.gw.scalar("SELECT count(*) FROM account"), 0);
}

#[test]
fn an_unbanned_account_has_no_ban_in_the_console() {
    let c = console("mod_rows");
    let _ = login(c.gw.port, "gina", "Gina");
    let id = id_of(&c, "gina");
    let (_, body) = admin(&c, "GET", &format!("/admin/accounts/{id}"), "");
    let detail = json(&body);
    assert!(detail["ban"].is_null(), "{detail}");
    assert_eq!(detail["reset_pending"], false, "{detail}");
    let (_, body) = admin(&c, "GET", "/admin/accounts", "");
    let row = &json(&body)["accounts"][0];
    assert!(row["ban"].is_null(), "{row}");
    assert_eq!(row["reset_pending"], false, "{row}");
}

fn deck(port: u16, token: &str) -> String {
    let body = r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

/// How many games `/health` says are running.
fn running(port: u16) -> i64 {
    let (status, body) = http(port, "GET", "/health", None, "");
    assert_eq!(status, 200, "{body}");
    json(&body)["games"]["running"].as_i64().expect("a count")
}

#[tokio::test(flavor = "multi_thread")]
async fn an_ended_game_tells_every_seat_why_and_is_over() {
    let c = console("mod_close");
    let port = c.gw.port;
    let _agent = attach_agent(&c.gw).await;
    let player = login(port, "hana", "Hana");

    // A table still waiting for its players cannot be ended; a stranger's
    // id finds nothing.
    let room = format!(
        r#"{{"deck_id":"{}","seats":2,"name":"Kitchen"}}"#,
        deck(port, &player)
    );
    let (status, body) = http(port, "POST", "/lobby/games", Some(&player), &room);
    assert_eq!(status, 200, "{body}");
    let waiting = json_field(&body, "game_id").to_owned();
    let (status, body) = admin(
        &c,
        "POST",
        &format!("/admin/games/{waiting}/close"),
        r#"{"reason":"too soon"}"#,
    );
    assert_eq!(status, 409, "{body}");
    let (status, _) = admin(
        &c,
        "POST",
        "/admin/games/no-such-game/close",
        r#"{"reason":"x"}"#,
    );
    assert_eq!(status, 404);

    let player2 = login(port, "ivan", "Ivan");
    let create = format!(r#"{{"deck_id":"{}","mode":"ai"}}"#, deck(port, &player2));
    let (status, body) = http(port, "POST", "/lobby/games", Some(&player2), &create);
    assert_eq!(status, 200, "a game against the house: {body}");
    let game = json_field(&body, "game_id").to_owned();
    let seat_token = json_field(&body, "seat_token").to_owned();
    let mut seat = common::dial_seat(port, &game, &seat_token).await;
    assert_eq!(running(port), 1);

    for (order, why) in [
        (r#"{"reason":""}"#, "empty"),
        (r#"{"reason":"a","extra":1}"#, "unknown field"),
        (r"{}", "none"),
    ] {
        let (status, body) = admin(&c, "POST", &format!("/admin/games/{game}/close"), order);
        assert_eq!(status, 400, "{why}: {body}");
    }
    assert_eq!(running(port), 1, "refused orders end nothing");

    let (status, body) = admin(
        &c,
        "POST",
        &format!("/admin/games/{game}/close"),
        r#"{"reason":"a rule dispute"}"#,
    );
    assert_eq!(status, 202, "{body}");

    let reason = tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(&mut seat).await {
                Some(v1::envelope::Msg::GameClosed(closed)) => return closed.reason,
                Some(_) => {}
                None => panic!("the seat socket closed before it was told"),
            }
        }
    })
    .await
    .expect("the seat was told the game was ended");
    assert_eq!(reason, "a rule dispute");

    for _ in 0..common::WAIT_TRIES {
        if running(port) == 0 {
            break;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    assert_eq!(running(port), 0, "the game ended as any other");
    // Ended: a second order finds a table no longer being played.
    let (status, _) = admin(
        &c,
        "POST",
        &format!("/admin/games/{game}/close"),
        r#"{"reason":"again"}"#,
    );
    assert_eq!(status, 409);
}

#[tokio::test(flavor = "multi_thread")]
async fn every_act_is_audited_without_a_token_or_the_reason() {
    let c = console("mod_audit");
    let port = c.gw.port;
    let _agent = attach_agent(&c.gw).await;
    let kept = login(port, "kurt", "Kurt");
    let doomed = login(port, "lena", "Lena");
    let (kurt, lena) = (id_of(&c, "kurt"), id_of(&c, "lena"));

    let (status, _) = admin(
        &c,
        "PUT",
        &format!("/admin/accounts/{kurt}/ban"),
        r#"{"reason":"a private grievance about Kurt"}"#,
    );
    assert_eq!(status, 200);
    let (status, _) = admin(&c, "DELETE", &format!("/admin/accounts/{kurt}/ban"), "");
    assert_eq!(status, 204);
    let (status, _) = admin(&c, "DELETE", &format!("/admin/accounts/{lena}"), "");
    assert_eq!(status, 204);

    let player2 = login(port, "mona", "Mona");
    let create = format!(r#"{{"deck_id":"{}","mode":"ai"}}"#, deck(port, &player2));
    let (_, body) = http(port, "POST", "/lobby/games", Some(&player2), &create);
    let game = json_field(&body, "game_id").to_owned();
    let seat_token = json_field(&body, "seat_token").to_owned();
    let _seat = common::dial_seat(port, &game, &seat_token).await;
    let (status, _) = admin(
        &c,
        "POST",
        &format!("/admin/games/{game}/close"),
        r#"{"reason":"a reason naming Mona"}"#,
    );
    assert_eq!(status, 202);

    let logs = plain(&c.gw.logs());
    for (action, id) in [
        ("account.ban", &kurt),
        ("account.unban", &kurt),
        ("account.delete", &lena),
        ("game.close", &game),
    ] {
        let line = logs
            .lines()
            .find(|l| {
                l.contains(&format!("action=\"{action}\""))
                    || l.contains(&format!("action={action} "))
            })
            .unwrap_or_else(|| panic!("no audit line for {action}:\n{logs}"));
        assert!(line.contains("baylee_gateway::audit"), "{line}");
        assert!(line.contains("admin=viktor"), "{line}");
        assert!(line.contains(id.as_str()), "{action} names {id}: {line}");
    }
    for secret in [
        "a private grievance",
        "a reason naming Mona",
        TOKEN,
        kept.as_str(),
        doomed.as_str(),
        player2.as_str(),
        seat_token.as_str(),
    ] {
        assert!(!logs.contains(secret), "{secret} in the log:\n{logs}");
    }
}
