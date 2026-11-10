//! End-to-end tests for the admin console's routes (`docs/protocol.md`
//! §"The admin console"): a real gateway with `BAYLEE_ADMIN_TOKEN`, its
//! console on a loopback port of its own, asked the way the feedback
//! service asks it.
//!
//! What they hold: the console is nowhere on the public port; a missing and
//! a wrong token get one answer; anything a browser marks is refused before
//! the token is read; ten wrong tokens shut it for everyone; the numbers
//! count what happened and name nobody; a change names its admin; and the
//! closed beta's key routes are gone.

#![allow(clippy::missing_docs_in_private_items)]

use crate::common;

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
        ("GET", "/admin/cards/blocks"),
        ("POST", "/admin/cards/blocks"),
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
            (
                401,
                r#"{"error":"gateway administrator authorization required"}"#
            )
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
        &[("Host", "gateway.baylee.app")][..],
    ] {
        let mut headers = vec![("Authorization", BEARER), ("X-Baylee-Admin", ADMIN)];
        headers.extend_from_slice(marks);
        for (method, path, body) in [
            ("GET", "/admin/stats", ""),
            ("POST", "/admin/cards/blocks", "{}"),
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
        "/admin/cards/blocks",
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
    let (_, body) = admin(&c, "GET", "/admin/cards/blocks", "");
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
    assert!(
        stats.get("invites").is_none() && stats["accounts"].get("admitted_by_key").is_none(),
        "the closed beta's keys are counted no more: {stats}"
    );
    assert_eq!(stats["gateway"]["registration"], "open", "{stats}");
    assert_eq!(
        stats["gateway"]["central"], false,
        "not connected to Baylee Central: {stats}"
    );
    assert!(
        stats["gateway"].get("mail").is_none(),
        "a gateway sends no mail: {stats}"
    );
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

/// The server metrics and the set progress are the shapes the UI reads:
/// the metrics answer on every platform (`/proc` fields `null` where there
/// is none) and name nobody; the sets partition the corpus, in release
/// order, and one set lists its cards with a Scryfall id the browser builds
/// a picture's address from itself (`docs/legal.md` §3).
#[test]
fn metrics_and_set_progress_have_the_shapes_the_console_reads() {
    let c = console("admin_metrics", &[]);
    // Requests on the public port are counted; the console's are not.
    for _ in 0..3 {
        let (status, _) = http(c.gw.port, "GET", "/health", None, "");
        assert_eq!(status, 200);
    }
    let (status, body) = admin(&c, "GET", "/admin/metrics", "");
    assert_eq!(status, 200, "{body}");
    let metrics = json(&body);
    assert_eq!(metrics["interval_secs"], 5, "{metrics}");
    assert_eq!(metrics["keep"], 720, "{metrics}");
    assert_eq!(metrics["platform"], std::env::consts::OS, "{metrics}");
    assert!(
        metrics["cores"].as_u64().is_some_and(|n| n >= 1),
        "{metrics}"
    );
    assert!(metrics["process_uptime_secs"].is_u64(), "{metrics}");
    assert!(metrics["host"].is_object(), "{metrics}");
    assert_eq!(metrics["host"]["disk_path"], "/", "{metrics}");
    if cfg!(target_os = "linux") {
        assert!(metrics["host"]["mem_total"].is_u64(), "{metrics}");
    }
    for column in [
        "at",
        "cpu",
        "load1",
        "mem_used",
        "disk_used",
        "net_in",
        "net_out",
        "sockets",
        "requests",
    ] {
        assert!(metrics["history"][column].is_array(), "{column}: {metrics}");
    }
    assert!(metrics["games"].is_array(), "{metrics}");
    // The first sample is taken as the gateway starts: a point with no rate.
    assert!(metrics["now"].is_object(), "{metrics}");
    assert_eq!(metrics["now"]["requests"], 0.0, "{metrics}");
    assert!(
        metrics["now"]["games"]
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    for leak in ["alice", "token", "@"] {
        assert!(!body.contains(leak), "{leak} in {body}");
    }

    let (status, body) = admin(&c, "GET", "/admin/sets", "");
    assert_eq!(status, 200, "{body}");
    let sets = json(&body);
    let pool = &sets["pool"];
    let corpus = sets["corpus"].as_u64().expect("the corpus size");
    let sum = [
        "implemented",
        "generated",
        "partial",
        "unimplemented",
        "absent",
    ]
    .iter()
    .map(|k| pool[k].as_u64().unwrap_or_else(|| panic!("{k}: {pool}")))
    .sum::<u64>();
    assert_eq!(sum, corpus, "{pool}");
    assert_eq!(
        pool["cards"].as_u64().unwrap() + pool["absent"].as_u64().unwrap(),
        corpus
    );
    let list = sets["sets"].as_array().expect("sets");
    assert_eq!(list[0]["code"], "lea", "{}", list[0]);
    assert_eq!(list[0]["position"], 0);
    assert_eq!(list[1]["code"], "leb");
    let per_set: u64 = list.iter().map(|s| s["total"].as_u64().unwrap()).sum();
    assert_eq!(per_set, corpus);
    assert!(sets["version"].is_string());

    let (status, body) = admin(&c, "GET", "/admin/sets/LEA", "");
    assert_eq!(status, 200, "{body}");
    let alpha = json(&body);
    assert_eq!(alpha["code"], "lea");
    let cards = alpha["cards"].as_array().expect("cards");
    assert_eq!(cards.len() as u64, alpha["total"].as_u64().unwrap());
    let bolt = cards
        .iter()
        .find(|c| c["name"] == "Lightning Bolt")
        .expect("Alpha has Lightning Bolt");
    assert_ne!(bolt["coverage"], "absent", "{bolt}");
    assert!(
        bolt["scryfall_id"]
            .as_str()
            .is_some_and(|id| id.len() == 36)
    );
    assert!(
        bolt["type_line"]
            .as_str()
            .is_some_and(|t| t.contains("Instant"))
    );
    assert!(bolt["index"].is_u64());
    for (path, status) in [("/admin/sets/zzzz", 404), ("/admin/sets/a-b", 400)] {
        let (got, body) = admin(&c, "GET", path, "");
        assert_eq!(got, status, "{path}: {body}");
    }
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

/// The one account a `tag=` lookup names, as the console answers it.
fn by_tag(c: &Console, tag: &str) -> serde_json::Value {
    let (status, body) = admin(c, "GET", &format!("/admin/accounts?tag={tag}"), "");
    assert_eq!(status, 200, "tag={tag}: {body}");
    json(&body)
}

/// A reported chat message names its sender by tag, and the console finds
/// exactly that account from it: registered or guest, with or without the
/// `#` and a name before it, and the padded `#0001` the handles draw is a
/// search that finds it too. A tag nobody holds finds nobody, and a value
/// that is no tag is refused.
#[test]
#[allow(clippy::too_many_lines)] // one set of accounts, looked up each way
fn an_account_is_found_by_its_exact_tag_and_by_the_padded_tag_in_its_handle() {
    let c = console("admin_tag", &[]);
    let port = c.gw.port;
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
    // A tag with letters in it, and one past four digits, on top of the
    // small ones the identity column handed out first.
    c.gw.sql("UPDATE account SET tag = 44803 WHERE username = 'bob'");
    c.gw.sql("UPDATE account SET tag = 65536 WHERE guest");

    let (_, body) = admin(&c, "GET", "/admin/accounts?sort=oldest", "");
    let all = json(&body);
    let rows = all["accounts"].as_array().unwrap();
    assert_eq!(rows.len(), 3, "{all}");
    let row = |name: &str| {
        rows.iter()
            .find(|r| r["display_name"] == name)
            .unwrap_or_else(|| panic!("{name}: {all}"))
            .clone()
    };
    let (alice, bob, visitor) = (row("Alice"), row("Bob"), row("Visitor"));
    assert_eq!(
        alice["handle"], "Alice#0001",
        "padded to four digits: {alice}"
    );
    assert_eq!(bob["handle"], "Bob#af03", "{bob}");
    assert_eq!(visitor["handle"], "Visitor#10000", "{visitor}");

    // Exactly that account, whichever way the tag is spelled, and the row
    // is the one a search gives.
    let (_, searched) = admin(&c, "GET", "/admin/accounts?q=bob", "");
    let searched = json(&searched);
    for spelled in ["af03", "AF03", "%23af03", "Bob%23af03", "Nobody%23af03"] {
        let found = by_tag(&c, spelled);
        assert_eq!(found["total"], 1, "tag={spelled}: {found}");
        assert_eq!(found["accounts"].as_array().unwrap().len(), 1, "{found}");
        assert_eq!(found["accounts"][0]["id"], bob["id"], "tag={spelled}");
        assert_eq!(
            found["accounts"][0], searched["accounts"][0],
            "the same row as a search gives, tag={spelled}"
        );
    }
    for field in [
        "handle", "online", "in_lobby", "playing", "guest", "username",
    ] {
        assert!(bob.get(field).is_some(), "{field} is in the row: {bob}");
    }

    // A small tag, with or without its padding; a guest; a long tag.
    for spelled in ["0001", "1", "%230001", "Alice%230001"] {
        let found = by_tag(&c, spelled);
        assert_eq!(found["total"], 1, "tag={spelled}: {found}");
        assert_eq!(found["accounts"][0]["id"], alice["id"], "tag={spelled}");
    }
    for spelled in ["10000", "Visitor%2310000"] {
        let found = by_tag(&c, spelled);
        assert_eq!(found["total"], 1, "tag={spelled}: {found}");
        assert_eq!(found["accounts"][0]["id"], visitor["id"], "tag={spelled}");
        assert_eq!(found["accounts"][0]["guest"], true, "{found}");
    }

    // A tag nobody holds finds nobody; the other filters still apply.
    let none = by_tag(&c, "beef");
    assert_eq!(none["total"], 0, "{none}");
    assert!(none["accounts"].as_array().unwrap().is_empty(), "{none}");
    let (_, body) = admin(&c, "GET", "/admin/accounts?tag=af03&kind=guest", "");
    assert_eq!(json(&body)["total"], 0, "Bob is no guest: {body}");

    // Not a tag, or too long to be anything.
    let too_long = "a".repeat(101);
    for bad in ["zz", "Bob%23", "%23", "af03%20x", "ffffffffff", &too_long] {
        let (status, body) = admin(&c, "GET", &format!("/admin/accounts?tag={bad}"), "");
        assert_eq!(status, 400, "tag={bad}: {body}");
    }
    // Blank is no filter at all.
    let (status, body) = admin(&c, "GET", "/admin/accounts?tag=", "");
    assert_eq!(status, 200, "{body}");
    assert_eq!(json(&body)["total"], 3, "{body}");

    // The tag a handle draws is a search that finds it. On the old query
    // `to_hex(1)` ('1') was compared with the typed '0001' and no padded
    // handle was ever found.
    for (typed, id) in [
        ("%230001", &alice["id"]),
        ("Alice%230001", &alice["id"]),
        ("%23af03", &bob["id"]),
        ("%23AF03", &bob["id"]),
        ("%2310000", &visitor["id"]),
    ] {
        let (_, body) = admin(&c, "GET", &format!("/admin/accounts?q={typed}"), "");
        let found = json(&body);
        assert_eq!(found["total"], 1, "q={typed}: {found}");
        assert_eq!(&found["accounts"][0]["id"], id, "q={typed}");
    }
    let (_, body) = admin(&c, "GET", "/admin/accounts?q=%230002", "");
    assert_eq!(json(&body)["total"], 0, "no account holds 0002: {body}");
}

#[test]
fn a_change_names_its_admin() {
    let c = console("admin_orders", &[]);
    let (status, _, body) = ask(
        c.port,
        "POST",
        "/admin/cards/blocks",
        &[("Authorization", BEARER)],
        "{}",
    );
    assert_eq!(status, 400, "{body}");
    let (status, _, _) = ask(
        c.port,
        "POST",
        "/admin/cards/blocks",
        &[("Authorization", BEARER), ("X-Baylee-Admin", "two words")],
        "{}",
    );
    assert_eq!(status, 400);
    let (_, listed) = admin(&c, "GET", "/admin/cards/blocks", "");
    assert_eq!(listed, "[]", "nothing refused was made");
}

/// The closed beta's keys are gone (10.11.2026): their routes are any
/// unknown path's `404`, the numbers count no key, and an account says
/// nothing of one.
#[test]
fn the_closed_betas_key_routes_are_gone() {
    let c = console("admin_no_keys", &[]);
    for (method, path, body) in [
        ("GET", "/admin/invites", ""),
        ("POST", "/admin/invites", r#"{"count":1}"#),
        (
            "DELETE",
            "/admin/invites/0190a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b",
            "",
        ),
    ] {
        let (status, answer) = admin(&c, method, path, body);
        assert_eq!(
            (status, answer.as_str()),
            (404, r#"{"error":"no such route"}"#),
            "{method} {path}"
        );
    }
    let (status, guest) = http(
        c.gw.port,
        "POST",
        "/auth/guest",
        None,
        r#"{"display_name":"Visitor"}"#,
    );
    assert_eq!(status, 200, "{guest}");
    let (_, body) = admin(&c, "GET", "/admin/accounts?kind=guest", "");
    let listed = json(&body);
    let row = &listed["accounts"][0];
    assert_eq!(row["display_name"], "Visitor", "{listed}");
    let (_, body) = admin(
        &c,
        "GET",
        &format!("/admin/accounts/{}", row["id"].as_str().unwrap()),
        "",
    );
    let detail = json(&body);
    for one in [row, &detail] {
        assert!(
            one.get("by_key").is_none() && one.get("key_note").is_none(),
            "{one}"
        );
    }
}

/// What the language-model collector asks to prove that a database is its
/// gateway's own (`crates/baylee-train`, `llm_collector4::teacher`): the
/// policy `nobody` for a profile no seat agent serves, set under a name it
/// made up, which the gateway writes as `updated_by` in its own database.
/// Pinned here because the collector's own tests stand a gateway in.
#[test]
fn the_collectors_probe_writes_the_name_it_was_given() {
    let c = console("admin_probe", &[]);
    for actor in [
        "llm-collector-0123456789abcdef0123456789abcdef",
        "llm-collector-fedcba9876543210fedcba9876543210",
    ] {
        let (status, _, body) = ask(
            c.port,
            "PUT",
            "/admin/llm/access/llm-collector-probe",
            &[
                ("Authorization", BEARER),
                ("X-Baylee-Admin", actor),
                ("Content-Type", "application/json"),
            ],
            r#"{"policy":"nobody"}"#,
        );
        assert_eq!(status, 200, "{body}");
        assert_eq!(
            c.gw.scalar(&format!(
                "SELECT count(*) FROM llm_profile_access \
                 WHERE profile_id = 'llm-collector-probe' AND policy = 'nobody' \
                   AND NOT default_for_registered AND updated_by = '{actor}'"
            )),
            1,
            "{actor}"
        );
    }
    assert_eq!(
        c.gw.scalar("SELECT count(*) FROM llm_profile_access"),
        1,
        "one row, set twice"
    );
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
        // Not a console's, but refused as early: a typo read as some mode
        // would open or shut a door nobody meant to.
        (vec![("BAYLEE_AUTH_MODE", "guest")], "BAYLEE_AUTH_MODE"),
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

/// Who comes in, chosen in the console (`docs/central-login.md` §"Who a
/// gateway lets in"): the default, `central`, until an admin chooses; a
/// choice is in force at the next sign-in, kept in the database across a
/// start, and audited; a session made before goes on. Unconnected and on
/// `central`, the gateway says it lets nobody in.
#[test]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
fn the_console_chooses_who_comes_in() {
    let c = console("authmode", &[("BAYLEE_AUTH_MODE", String::new())]);
    let read = |c: &Console| -> serde_json::Value {
        let (status, body) = admin(c, "GET", "/admin/auth-mode", "");
        assert_eq!(status, 200, "{body}");
        serde_json::from_str(&body).unwrap()
    };
    let guest = |c: &Console| http(c.gw.port, "POST", "/auth/guest", None, "{}");
    let first = read(&c);
    assert_eq!(first["mode"], "central", "{first}");
    assert_eq!(first["source"], "default", "{first}");
    assert_eq!(first["set_by"], serde_json::Value::Null, "{first}");
    assert_eq!(first["connected"], false, "nobody comes in: {first}");
    let (_, stats) = admin(&c, "GET", "/admin/stats", "");
    let stats: serde_json::Value = serde_json::from_str(&stats).unwrap();
    assert_eq!(stats["gateway"]["auth_mode"], "central", "{stats}");
    assert_eq!(stats["gateway"]["auth_mode_source"], "default", "{stats}");
    assert_eq!(stats["guests"]["enabled"], false, "{stats}");
    assert_eq!(guest(&c).0, 403);

    for body in [
        r#"{"mode":"guest"}"#,
        r#"{"mode":"Mixed"}"#,
        r#"{"mode":"mixed","admin":"someone"}"#,
        "{}",
        "not json",
    ] {
        let (status, answer) = admin(&c, "PUT", "/admin/auth-mode", body);
        assert_eq!(status, 400, "{body}: {answer}");
    }
    assert_eq!(
        c.gw.scalar("SELECT count(*) FROM auth_mode"),
        0,
        "nothing chosen"
    );

    let (status, body) = admin(&c, "PUT", "/admin/auth-mode", r#"{"mode":"guests"}"#);
    assert_eq!(status, 200, "{body}");
    let chosen: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(chosen["mode"], "guests", "{chosen}");
    assert_eq!(chosen["source"], "console", "{chosen}");
    assert_eq!(chosen["set_by"], ADMIN, "{chosen}");
    assert!(
        chosen["set_at"]
            .as_str()
            .is_some_and(|at| at.ends_with('Z')),
        "{chosen}"
    );
    let (status, answer) = guest(&c);
    assert_eq!(status, 200, "in force at the next sign-in: {answer}");
    let early = common::json_field(&answer, "token").to_owned();
    let (_, config) = http(c.gw.port, "GET", "/auth/config", None, "");
    assert!(config.contains("\"auth_mode\":\"guests\""), "{config}");

    // Back to the default: a guest already in stays, the next is refused.
    let (status, body) = admin(&c, "PUT", "/admin/auth-mode", r#"{"mode":"central"}"#);
    assert_eq!(status, 200, "{body}");
    assert_eq!(http(c.gw.port, "GET", "/me", Some(&early), "").0, 200);
    assert_eq!(guest(&c).0, 403);
    let (status, body) = admin(&c, "PUT", "/admin/auth-mode", r#"{"mode":"mixed"}"#);
    assert_eq!(status, 200, "{body}");
    assert_eq!(c.gw.text("SELECT mode FROM auth_mode"), "mixed");
    assert_eq!(c.gw.text("SELECT set_by FROM auth_mode"), ADMIN);

    // Read again at the next start, from the same database.
    let again = console(
        "authmode_again",
        &[
            ("BAYLEE_AUTH_MODE", String::new()),
            ("DATABASE_URL", c.gw.database_url()),
        ],
    );
    let kept = read(&again);
    assert_eq!(
        (kept["mode"].as_str(), kept["source"].as_str()),
        (Some("mixed"), Some("console"))
    );
    assert_eq!(guest(&again).0, 200);

    let logs = plain(&c.gw.logs());
    let lines: Vec<&str> = logs
        .lines()
        .filter(|line| line.contains("action=\"auth_mode.set\""))
        .collect();
    assert_eq!(lines.len(), 3, "{logs}");
    assert!(lines.iter().all(|line| line.contains(ADMIN)), "{lines:?}");
}

/// The operator's `BAYLEE_AUTH_MODE` wins: the console shows it, and a
/// choice there is refused and changes nothing.
#[test]
fn the_operators_auth_mode_locks_the_console() {
    let c = console("authmode_env", &[("BAYLEE_AUTH_MODE", "guests".into())]);
    let (status, body) = admin(&c, "GET", "/admin/auth-mode", "");
    assert_eq!(status, 200, "{body}");
    let shown: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        (shown["mode"].as_str(), shown["source"].as_str()),
        (Some("guests"), Some("environment"))
    );
    let (status, body) = admin(&c, "PUT", "/admin/auth-mode", r#"{"mode":"central"}"#);
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("BAYLEE_AUTH_MODE"), "{body}");
    assert_eq!(c.gw.scalar("SELECT count(*) FROM auth_mode"), 0);
    assert_eq!(http(c.gw.port, "POST", "/auth/guest", None, "{}").0, 200);
}

/// The terms of use in the console (`docs/protocol.md` §"Terms of use
/// (WG-1)"): the default terms until an admin publishes a version; a
/// published version is in force at once, never changes, and is listed with
/// who published it. Without Baylee Central nothing confirms it there.
#[test]
fn the_console_publishes_terms_of_use() {
    let c = console("terms", &[("BAYLEE_TERMS_PATH", String::new())]);
    let (status, body) = admin(&c, "GET", "/admin/terms", "");
    assert_eq!(status, 200, "{body}");
    let current: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(current["terms"]["source"], "default", "{body}");
    assert_eq!(current["terms"]["at_central"], false, "no Central here");
    let langs: Vec<&str> = current["terms"]["texts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["lang"].as_str().unwrap())
        .collect();
    assert_eq!(langs, ["de", "en"]);

    let publish = |version: &str, texts: serde_json::Value| {
        admin(
            &c,
            "POST",
            "/admin/terms",
            &serde_json::json!({"version": version, "texts": texts}).to_string(),
        )
    };
    let texts = serde_json::json!([
        {"lang": "en", "markdown": "# Terms\n\nBe kind.\n"},
        {"lang": "de", "updated": "2026-10-20", "markdown": "# Bedingungen\n\nSei nett.\n"},
    ]);
    let default_version = current["terms"]["version"].as_str().unwrap().to_owned();
    let (status, body) = publish(&default_version, texts.clone());
    assert_eq!(
        status, 409,
        "the default's version is in force already: {body}"
    );
    let (status, body) = publish("admin-1", texts.clone());
    assert_eq!(status, 201, "{body}");
    let published: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(published["terms"]["source"], "published");
    assert_eq!(published["terms"]["version"], "admin-1");
    let (_, served) = http(c.gw.port, "GET", "/terms?lang=de", None, "");
    let served: serde_json::Value = serde_json::from_str(&served).unwrap();
    assert_eq!(served["version"], "admin-1", "in force at once");
    assert_eq!(served["markdown"], "# Bedingungen\n\nSei nett.\n");
    assert_eq!(served["updated"], "2026-10-20");
    assert_eq!(served["digest"], published["terms"]["digest"]);

    let (status, body) = publish(
        "admin-1",
        serde_json::json!([{"lang": "en", "markdown": "# Changed\n"}]),
    );
    assert_eq!(status, 409, "a published version never changes: {body}");
    for (version, texts, why) in [
        (
            "admin-2",
            serde_json::json!([{"lang": "de", "markdown": "# T\n"}]),
            "`en`",
        ),
        (
            "draft",
            serde_json::json!([{"lang": "en", "markdown": "# T\n"}]),
            "draft",
        ),
        (
            "two words",
            serde_json::json!([{"lang": "en", "markdown": "# T\n"}]),
            "version",
        ),
        (
            "admin-2",
            serde_json::json!([{"lang": "en", "markdown": "a\u{7}b"}]),
            "control character",
        ),
    ] {
        let (status, body) = publish(version, texts);
        assert_eq!(status, 400, "{version}: {body}");
        assert!(body.contains(why), "{version}: {body}");
    }
    let (status, body) = admin(&c, "GET", "/admin/terms/versions", "");
    assert_eq!(status, 200);
    let versions: serde_json::Value = serde_json::from_str(&body).unwrap();
    let versions = versions["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 1, "{body}");
    assert_eq!(versions[0]["version"], "admin-1");
    assert_eq!(versions[0]["published_by"], ADMIN);
}
