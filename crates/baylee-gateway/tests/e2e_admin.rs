//! End-to-end tests for the admin console's routes (`docs/protocol.md`
//! §"The admin console"): a real gateway with `BAYLEE_ADMIN_TOKEN`, its
//! console on a loopback port of its own, asked the way the feedback
//! service asks it.
//!
//! What they hold: the console is nowhere on the public port; a missing and
//! a wrong token get one answer; anything a browser marks is refused before
//! the token is read; ten wrong tokens shut it for everyone; the numbers
//! count what happened; and keys made, listed and revoked here are the keys
//! `baylee-gateway invite` makes, lists and revokes, with an audit line for
//! every change that holds no key.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{Gateway, http, spawn_gateway, spawn_gateway_with};

/// The token every console here is started with.
const TOKEN: &str = "e2e-admin-token-0123456789abcdef0123456789";

/// The admin the feedback service would name.
const ADMIN: &str = "viktor";

/// A gateway and its console's port.
struct Console {
    gw: Gateway,
    port: u16,
}

/// A gateway with a console, and `extra` environment.
fn console(label: &str, extra: &[(&str, String)]) -> Console {
    let port_file = std::env::temp_dir().join(format!(
        "baylee-admin-{label}-{}-{}.port",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&port_file);
    let mut env = vec![
        ("BAYLEE_ADMIN_TOKEN", TOKEN.to_owned()),
        ("BAYLEE_ADMIN_BIND", "127.0.0.1:0".to_owned()),
        (
            "BAYLEE_ADMIN_PORT_FILE",
            port_file.to_string_lossy().into_owned(),
        ),
        ("RUST_LOG", "baylee_gateway::audit=info,warn".to_owned()),
    ];
    env.extend(extra.iter().map(|(k, v)| (*k, v.clone())));
    let gw = spawn_gateway_with(label, &env);
    // Written before the gateway's own port file, which the harness waited
    // for.
    let port = std::fs::read_to_string(&port_file)
        .expect("the console's port file")
        .trim()
        .parse()
        .expect("a port");
    let _ = std::fs::remove_file(&port_file);
    Console { gw, port }
}

/// One request: status, the head (lower case) and the body.
fn ask(
    port: u16,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (u16, String, String) {
    use std::fmt::Write as _;
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let mut head = format!("{method} {path} HTTP/1.1\r\n");
    if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
        let _ = write!(head, "Host: 127.0.0.1:{port}\r\n");
    }
    for (name, value) in headers {
        let _ = write!(head, "{name}: {value}\r\n");
    }
    let request = format!(
        "{head}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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
    let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
    (status, head.to_lowercase(), body.to_owned())
}

const BEARER: &str = "Bearer e2e-admin-token-0123456789abcdef0123456789";

/// A console request as the feedback service sends it.
fn admin(c: &Console, method: &str, path: &str, body: &str) -> (u16, String) {
    let (status, _, body) = ask(
        c.port,
        method,
        path,
        &[("Authorization", BEARER), ("X-Baylee-Admin", ADMIN)],
        body,
    );
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

/// `baylee-gateway invite <args>` against this gateway's schema.
fn cli(gw: &Gateway, args: &[&str]) -> (i32, String, String) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
        .arg("invite")
        .args(args)
        .env("DATABASE_URL", gw.database_url())
        .output()
        .expect("the command runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn without_a_token_there_is_no_console_and_the_public_port_never_has_one() {
    let plain = spawn_gateway("admin_off");
    let (status, _, _) = ask(
        plain.port,
        "GET",
        "/admin/stats",
        &[("Authorization", BEARER)],
        "",
    );
    assert_eq!(status, 404, "a gateway without a token has no console");

    let c = console("admin_public", &[]);
    for (method, path) in [
        ("GET", "/admin/stats"),
        ("GET", "/admin/invites"),
        ("POST", "/admin/invites"),
    ] {
        let (status, _, body) = ask(
            c.gw.port,
            method,
            path,
            &[("Authorization", BEARER), ("X-Baylee-Admin", ADMIN)],
            "{}",
        );
        assert_eq!(status, 404, "{method} {path} on the public port: {body}");
    }
    let (status, _) = admin(&c, "GET", "/admin/stats", "");
    assert_eq!(status, 200, "and the console's own port answers");
}

#[test]
fn a_missing_and_a_wrong_token_get_one_answer() {
    let c = console("admin_token", &[]);
    let wrong_same_length = format!("Bearer {}", "x".repeat(TOKEN.len()));
    let one_off = format!("Bearer {}", &TOKEN[..TOKEN.len() - 1]);
    let longer = format!("{BEARER}0");
    let answers: Vec<(u16, String)> = [
        None,
        Some("Bearer "),
        Some(wrong_same_length.as_str()),
        Some(one_off.as_str()),
        Some(longer.as_str()),
        Some(TOKEN),
    ]
    .into_iter()
    .map(|auth| {
        let headers: Vec<(&str, &str)> = auth.map(|a| ("Authorization", a)).into_iter().collect();
        let (status, _, body) = ask(c.port, "GET", "/admin/stats", &headers, "");
        (status, body)
    })
    .collect();
    for (status, body) in &answers {
        assert_eq!(
            (*status, body.as_str()),
            (401, r#"{"error":"the admin token is needed"}"#)
        );
    }
    let (status, body) = admin(&c, "GET", "/admin/stats", "");
    assert_eq!(status, 200, "{body}");
}

#[test]
fn a_browser_and_a_strange_host_are_refused_before_the_token_is_read() {
    let c = console("admin_browser", &[]);
    for marks in [
        &[("Origin", "http://127.0.0.1")][..],
        &[("Origin", "null")][..],
        &[("Sec-Fetch-Site", "same-origin")][..],
        &[("Sec-Fetch-Mode", "cors")][..],
        &[("Host", "evil.example")][..],
        &[("Host", "baylee.acevik.de")][..],
    ] {
        let mut headers = vec![("Authorization", BEARER), ("X-Baylee-Admin", ADMIN)];
        headers.extend_from_slice(marks);
        for (method, path, body) in [
            ("GET", "/admin/stats", ""),
            ("POST", "/admin/invites", "{}"),
        ] {
            let (status, head, body) = ask(c.port, method, path, &headers, body);
            assert_eq!(status, 403, "{marks:?} {method} {path}: {body}");
            assert!(!head.contains("access-control-allow"), "{head}");
        }
    }
    // A preflight is neither answered nor given a CORS header.
    let (status, head, _) = ask(
        c.port,
        "OPTIONS",
        "/admin/invites",
        &[
            ("Origin", "https://evil.example"),
            ("Access-Control-Request-Method", "POST"),
        ],
        "",
    );
    assert_eq!(status, 403);
    assert!(!head.contains("access-control-allow"), "{head}");
    // And nothing it answers is kept on the way.
    let (status, head, _) = ask(
        c.port,
        "GET",
        "/admin/stats",
        &[("Authorization", BEARER)],
        "",
    );
    assert_eq!(status, 200);
    assert!(head.contains("cache-control: no-store"), "{head}");
    assert!(!head.contains("access-control-allow"), "{head}");
    // Nothing was made by any of the refused requests.
    let (_, body) = admin(&c, "GET", "/admin/invites", "");
    assert_eq!(body, "[]");
}

#[test]
fn ten_wrong_tokens_shut_the_console_for_everyone_for_a_while() {
    let c = console("admin_limit", &[]);
    let wrong = format!("Bearer {}", "y".repeat(40));
    for n in 0..10 {
        let (status, _, _) = ask(
            c.port,
            "GET",
            "/admin/stats",
            &[("Authorization", &wrong)],
            "",
        );
        assert_eq!(status, 401, "attempt {n}");
    }
    let (status, head, _) = ask(
        c.port,
        "GET",
        "/admin/stats",
        &[("Authorization", BEARER)],
        "",
    );
    assert_eq!(status, 429, "the right token waits too");
    assert!(head.contains("retry-after: 300"), "{head}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_numbers_count_what_happened_and_name_nobody() {
    let c = console("admin_stats", &[("BAYLEE_GUEST_CAP", "25".into())]);
    let port = c.gw.port;
    let (_, body) = admin(&c, "GET", "/admin/stats", "");
    let before = json(&body);
    assert_eq!(before["accounts"]["registered"], 0, "{before}");
    assert_eq!(before["guests"]["live"], 0, "{before}");
    assert_eq!(before["guests"]["cap"], 25, "{before}");
    assert_eq!(before["agents"]["connected"], 0, "{before}");

    let _alice = common::login(port, "alice", "Alice");
    let _bob = common::login(port, "bob", "Bob");
    let (status, guest) = http(
        port,
        "POST",
        "/auth/guest",
        None,
        r#"{"display_name":"Visitor"}"#,
    );
    assert_eq!(status, 200, "{guest}");
    let (status, _) = admin(&c, "POST", "/admin/invites", r#"{"count":3,"uses":2}"#);
    assert_eq!(status, 201);
    let agent = common::attach_agent(&c.gw).await;

    let (status, body) = admin(&c, "GET", "/admin/stats", "");
    assert_eq!(status, 200, "{body}");
    let stats = json(&body);
    assert_eq!(stats["accounts"]["registered"], 2, "{stats}");
    assert_eq!(stats["accounts"]["created"]["today_utc"], 2, "{stats}");
    assert_eq!(stats["accounts"]["created"]["last_7d"], 2, "{stats}");
    assert_eq!(stats["accounts"]["created"]["last_30d"], 2, "{stats}");
    assert_eq!(stats["accounts"]["confirmed_email"], 0, "{stats}");
    assert_eq!(stats["guests"]["live"], 1, "{stats}");
    assert_eq!(stats["guests"]["enabled"], true, "{stats}");
    // Two logins and a guest, each a session of its own account.
    assert_eq!(stats["online"]["sessions_live"], 3, "{stats}");
    assert_eq!(stats["online"]["accounts_signed_in"], 3, "{stats}");
    assert_eq!(
        stats["online"]["players"], 0,
        "nobody has a lobby open: {stats}"
    );
    assert_eq!(stats["games"]["running"], 0, "{stats}");
    assert_eq!(stats["games"]["recorded"], 0, "{stats}");
    assert_eq!(stats["agents"]["connected"], 1, "{stats}");
    assert_eq!(stats["invites"]["total"], 3, "{stats}");
    assert_eq!(stats["invites"]["active"], 3, "{stats}");
    assert_eq!(stats["invites"]["uses_left"], 6, "{stats}");
    assert_eq!(stats["gateway"]["registration"], "open", "{stats}");
    assert!(stats["gateway"]["version"].is_string(), "{stats}");
    assert!(
        stats["at"].as_str().is_some_and(|at| at.ends_with('Z')),
        "{stats}"
    );

    // Aggregates only: no name, username or id of anybody anywhere in it.
    for leak in ["alice", "Alice", "bob", "Visitor", "token", "#"] {
        assert!(!body.contains(leak), "{leak} in {body}");
    }
    agent.abort();
}

/// The accounts, one account and the live view name players (owner,
/// 09.10.2026), and never a secret: no password hash, token, session hash
/// or e-mail address.
#[tokio::test(flavor = "multi_thread")]
async fn accounts_and_tables_are_listed_by_handle_and_nothing_secret() {
    let c = console("admin_accounts", &[]);
    let port = c.gw.port;
    let _agent = common::attach_agent(&c.gw).await;
    let alice = common::login(port, "alice", "Alice");
    let _bob = common::login(port, "bob", "Bob");
    let (status, guest) = http(
        port,
        "POST",
        "/auth/guest",
        None,
        r#"{"display_name":"Visitor"}"#,
    );
    assert_eq!(status, 200, "{guest}");
    let deck = r#"{"name":"Woods","cards":["40 Forest","20 Swamp"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(&alice), deck);
    assert_eq!(status, 200, "{body}");
    let deck_id = common::json_field(&body, "deck_id").to_owned();
    let room = format!(r#"{{"deck_id":"{deck_id}","seats":2,"name":"Kitchen table"}}"#);
    let (status, body) = http(port, "POST", "/lobby/games", Some(&alice), &room);
    assert_eq!(status, 200, "{body}");
    let game_id = common::json_field(&body, "game_id").to_owned();

    let (status, body) = admin(&c, "GET", "/admin/accounts", "");
    assert_eq!(status, 200, "{body}");
    let all = json(&body);
    assert_eq!(all["total"], 3, "{all}");
    assert_eq!(all["online"], 1, "Alice sits at a table: {all}");
    let rows = all["accounts"].as_array().unwrap();
    assert_eq!(rows[0]["display_name"], "Visitor", "newest first: {all}");
    let alice_row = rows.iter().find(|r| r["username"] == "alice").unwrap();
    assert_eq!(alice_row["decks"], 1, "{alice_row}");
    assert_eq!(alice_row["sessions"], 1, "{alice_row}");
    assert_eq!(alice_row["online"], true, "{alice_row}");
    assert_eq!(alice_row["guest"], false, "{alice_row}");
    assert!(alice_row["active_at"].is_string(), "{alice_row}");
    assert!(
        alice_row["handle"]
            .as_str()
            .is_some_and(|h| h.starts_with("Alice#")),
        "{alice_row}"
    );
    for secret in ["password", "token", "hash", "\"email\""] {
        assert!(!body.contains(secret), "{secret} in {body}");
    }

    let (_, body) = admin(&c, "GET", "/admin/accounts?kind=guest", "");
    let guests = json(&body);
    assert_eq!(guests["total"], 1, "{guests}");
    assert_eq!(guests["accounts"][0]["guest"], true, "{guests}");
    let (_, body) = admin(&c, "GET", "/admin/accounts?online=true", "");
    assert_eq!(json(&body)["total"], 1, "{body}");
    let (_, body) = admin(&c, "GET", "/admin/accounts?q=BO&sort=name", "");
    let found = json(&body);
    assert_eq!(found["total"], 1, "{found}");
    assert_eq!(found["accounts"][0]["username"], "bob", "{found}");
    let (_, body) = admin(
        &c,
        "GET",
        "/admin/accounts?limit=1&offset=1&sort=oldest",
        "",
    );
    let page = json(&body);
    assert_eq!(page["total"], 3, "{page}");
    assert_eq!(page["accounts"][0]["username"], "bob", "{page}");
    for bad in ["?kind=admins", "?sort=a.id", "?limit=x"] {
        let (status, body) = admin(&c, "GET", &format!("/admin/accounts{bad}"), "");
        assert_eq!(status, 400, "{bad}: {body}");
    }

    let id = alice_row["id"].as_str().unwrap();
    let (status, body) = admin(&c, "GET", &format!("/admin/accounts/{id}"), "");
    assert_eq!(status, 200, "{body}");
    let detail = json(&body);
    assert_eq!(detail["deck_list"][0]["name"], "Woods", "{detail}");
    assert_eq!(detail["deck_list"][0]["cards"], 60, "{detail}");
    assert_eq!(detail["table"]["id"], game_id.as_str(), "{detail}");
    assert_eq!(detail["table"]["state"], "waiting", "{detail}");
    assert!(!body.contains("Forest"), "no deck's cards: {body}");
    let (status, _) = admin(
        &c,
        "GET",
        "/admin/accounts/0199aaaa-0000-7000-8000-000000000001",
        "",
    );
    assert_eq!(status, 404);
    let (status, _) = admin(&c, "GET", "/admin/accounts/nobody", "");
    assert_eq!(status, 400);

    live_and_days(&c, &game_id);
}

/// The live view and the days, with Alice's room open at `game_id`.
fn live_and_days(c: &Console, game_id: &str) {
    let (status, body) = admin(c, "GET", "/admin/live", "");
    assert_eq!(status, 200, "{body}");
    let live = json(&body);
    let table = &live["tables"][0];
    assert_eq!(table["name"], "Kitchen table", "{live}");
    assert_eq!(table["state"], "waiting", "{live}");
    assert!(
        table["host"]
            .as_str()
            .is_some_and(|h| h.starts_with("Alice#")),
        "{live}"
    );
    assert_eq!(table["seats"].as_array().unwrap().len(), 2, "{live}");
    assert_eq!(live["players"][0]["waiting"], game_id, "{live}");
    assert_eq!(live["agents"].as_array().unwrap().len(), 1, "{live}");

    let (_, body) = admin(c, "GET", "/admin/stats", "");
    let stats = json(&body);
    assert_eq!(stats["accounts"]["decks"], 1, "{stats}");
    assert_eq!(stats["guests"]["created"]["today_utc"], 1, "{stats}");
    let daily = stats["daily"].as_array().unwrap();
    assert_eq!(daily.len(), 30, "{stats}");
    assert_eq!(daily[29]["registered"], 2, "today is last: {stats}");
    assert_eq!(daily[29]["guests"], 1, "{stats}");
}

#[test]
fn a_change_names_its_admin_and_is_refused_in_the_commands_words() {
    let c = console("admin_orders", &[]);
    // A change names its admin.
    let (status, _, body) = ask(
        c.port,
        "POST",
        "/admin/invites",
        &[("Authorization", BEARER)],
        "{}",
    );
    assert_eq!(status, 400, "{body}");
    let (status, _, _) = ask(
        c.port,
        "POST",
        "/admin/invites",
        &[("Authorization", BEARER), ("X-Baylee-Admin", "two words")],
        "{}",
    );
    assert_eq!(status, 400);

    // Refused in the command's own words.
    for (body, why) in [
        (r#"{"uses":0}"#, r#"--uses takes 1 to 1000, not \"0\""#),
        (r#"{"count":101}"#, r#"--count takes 1 to 100, not \"101\""#),
        (
            r#"{"expires":"30w"}"#,
            r#"--expires takes days or hours, like 30d or 12h, not \"30w\""#,
        ),
        (
            r#"{"note":"two\nlines"}"#,
            "a note is one line of plain text",
        ),
    ] {
        let (status, answer) = admin(&c, "POST", "/admin/invites", body);
        assert_eq!(status, 400, "{body}: {answer}");
        assert_eq!(answer, format!(r#"{{"error":"{why}"}}"#), "{body}");
    }
    for body in [r#"{"colour":"red"}"#, "not json", r#"{"uses":"two"}"#] {
        let (status, answer) = admin(&c, "POST", "/admin/invites", body);
        assert_eq!(status, 400, "{body}: {answer}");
    }
    let (_, listed) = admin(&c, "GET", "/admin/invites", "");
    assert_eq!(listed, "[]", "nothing refused was made");
}

#[test]
fn keys_made_here_are_the_commands_keys_and_every_change_is_audited() {
    let c = console("admin_invites", &[("BAYLEE_REGISTRATION", "invite".into())]);
    let port = c.gw.port;

    let (status, body) = admin(
        &c,
        "POST",
        "/admin/invites",
        r#"{"count":2,"uses":1,"expires":"30d","note":"  for Max  "}"#,
    );
    assert_eq!(status, 201, "{body}");
    let made = json(&body);
    assert_eq!(made["uses"], 1);
    assert_eq!(made["note"], "for Max");
    assert!(made["expires_at"].is_string(), "{made}");
    let keys: Vec<(String, String)> = made["keys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| {
            (
                k["id"].as_str().unwrap().to_owned(),
                k["key"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(keys.len(), 2);
    for (_, key) in &keys {
        assert!(key.starts_with("BAYLEE-") && key.len() == 26, "{key}");
    }

    // A key the console made lets a player in, once.
    let register = |name: &str, key: &str| {
        let body = serde_json::json!({
            "username": name,
            "display_name": name,
            "password": "a-very-fine-password",
            "invite_key": key,
        });
        http(port, "POST", "/auth/register", None, &body.to_string()).0
    };
    assert_eq!(register("carol", &keys[0].1), 200);
    assert_eq!(register("dave", &keys[0].1), 403, "one use");

    // The command lists the console's keys, and the console the command's.
    let (code, out, err) = cli(&c.gw, &["create", "--note", "from the shell"]);
    assert_eq!(code, 0, "{err}");
    let shell_key = out.trim().to_owned();
    let (code, table, _) = cli(&c.gw, &["list"]);
    assert_eq!(code, 0);
    for (id, key) in &keys {
        assert!(table.contains(id.as_str()), "{table}");
        assert!(!table.contains(key.as_str()), "{table}");
    }
    let (status, body) = admin(&c, "GET", "/admin/invites", "");
    assert_eq!(status, 200);
    assert!(!body.contains(&keys[0].1) && !body.contains(&keys[1].1) && !body.contains(&shell_key));
    let listed = json(&body);
    let listed = listed.as_array().unwrap();
    assert_eq!(listed.len(), 3, "{listed:?}");
    assert_eq!(listed[0]["note"], "from the shell", "newest first");
    let first = listed
        .iter()
        .find(|k| k["id"] == keys[0].0.as_str())
        .unwrap();
    assert_eq!(first["admitted"], 1, "{first}");
    assert_eq!(first["uses_left"], 0, "{first}");
    assert_eq!(first["state"], "used_up", "{first}");
    assert_eq!(first["note"], "for Max", "{first}");

    // Revoked here, refused at the door, and shown revoked by the command.
    let (status, body) = admin(&c, "DELETE", &format!("/admin/invites/{}", keys[1].0), "");
    assert_eq!(status, 204, "{body}");
    let (status, body) = admin(&c, "DELETE", &format!("/admin/invites/{}", keys[1].0), "");
    assert_eq!(status, 404);
    assert_eq!(
        body,
        format!(
            r#"{{"error":"no key {} that is not revoked already"}}"#,
            keys[1].0
        )
    );
    let (status, _) = admin(&c, "DELETE", "/admin/invites/not-a-uuid", "");
    assert_eq!(status, 400);
    assert_eq!(register("erin", &keys[1].1), 403, "revoked");
    let (_, table, _) = cli(&c.gw, &["list"]);
    let line = table
        .lines()
        .find(|l| l.starts_with(keys[1].0.as_str()))
        .unwrap();
    assert!(!line.contains("  -  "), "a revocation time: {line}");
    let (_, body) = admin(&c, "GET", "/admin/invites", "");
    assert!(body.contains(r#""state":"revoked""#), "{body}");
    // Revoked by the command, seen here.
    let shell_id = json(&body)
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["note"] == "from the shell")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (code, _, _) = cli(&c.gw, &["revoke", &shell_id]);
    assert_eq!(code, 0);
    let (status, _) = admin(&c, "DELETE", &format!("/admin/invites/{shell_id}"), "");
    assert_eq!(status, 404, "the command revoked it first");

    audited(&c.gw, &keys, &keys[1].0);
}

/// Every change has its line: who, what, which ids; never a key or the
/// token, nor the note, which may name a person.
fn audited(gw: &Gateway, keys: &[(String, String)], revoked: &str) {
    let logs = plain(&gw.logs());
    let made_line = logs
        .lines()
        .find(|l| l.contains("invite.create"))
        .unwrap_or_else(|| panic!("no audit line for the keys:\n{logs}"));
    assert!(made_line.contains("admin=viktor"), "{made_line}");
    assert!(made_line.contains("baylee_gateway::audit"), "{made_line}");
    for (id, _) in keys {
        assert!(made_line.contains(id.as_str()), "{made_line}");
    }
    let revoke_line = logs
        .lines()
        .find(|l| l.contains("invite.revoke"))
        .unwrap_or_else(|| panic!("no audit line for the revocation:\n{logs}"));
    assert!(revoke_line.contains(revoked), "{revoke_line}");
    assert!(revoke_line.contains("admin=viktor"), "{revoke_line}");
    let secrets = keys.iter().map(|(_, key)| key.as_str());
    for secret in secrets.chain([TOKEN, "for Max"]) {
        assert!(!logs.contains(secret), "{secret} in the log:\n{logs}");
    }
}

/// The gateway refuses to start with a console it cannot keep safe, and
/// says why, before it touches the database.
#[test]
fn a_console_that_could_be_misread_stops_the_start() {
    let agent = "an-agent-token-that-is-long-enough-000000";
    for (env, why) in [
        (
            vec![("BAYLEE_ADMIN_TOKEN", "short")],
            "a token needs at least 32 characters",
        ),
        (
            vec![("BAYLEE_ADMIN_TOKEN", agent), ("BAYLEE_AGENT_TOKEN", agent)],
            "already another secret",
        ),
        (
            vec![
                ("BAYLEE_ADMIN_TOKEN", TOKEN),
                ("BAYLEE_ADMIN_BIND", "0.0.0.0:28767"),
            ],
            "not a loopback address",
        ),
        (
            vec![("BAYLEE_ADMIN_BIND", "127.0.0.1:28767")],
            "BAYLEE_ADMIN_TOKEN is not",
        ),
    ] {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("DATABASE_URL", "postgres://nobody@127.0.0.1:1/nothing")
            .envs(env.iter().copied())
            .output()
            .expect("the gateway runs");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{env:?} started");
        assert!(stderr.contains(why), "{env:?}: {stderr}");
    }
}
