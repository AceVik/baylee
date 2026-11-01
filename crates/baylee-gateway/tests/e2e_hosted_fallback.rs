//! A hosted profile's fallback (`Definition::fallback`, `Reported::diverts`),
//! through a real gateway and a real seat agent (`baylee_seathost::Host`):
//! the console refuses a fallback that cannot stand by and shows one that
//! can; a profile at its `max_games` stays on offer while its fallback is
//! free, an order for it is taken (`202`) and the seat agent's launcher is
//! handed a game that plays the fallback in the ordered profile's chair; and
//! it is refused again (`409`) once the fallback is full too.
//!
//! No model is asked and no bridge runs: the seat agent's launcher records the
//! jobs it is handed and sits until it is stopped, as the unit tests of the
//! seat agent do, so what is held here is the gateway's half of the feature
//! (the console's refusals, `Registry::pick`, the rows) and that the two ends
//! agree on a diversion.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_client_core::llmseat::keys::MemoryKeys;
use baylee_protocol::seathost::Definition;
use baylee_seat::deck::Deck;
use baylee_seat::lobby::{Lobby, Session};
use baylee_seathost::launch::{Exit, Launcher, Running, SeatJob};
use baylee_seathost::state::Probe;
use baylee_seathost::{Config, Host, write_profiles};
use common::{Gateway, attach_agent, http, login, spawn_gateway_with};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

const SEATHOST_TOKEN: &str = "e2e-seathost-token-0123456789abcdef0123";
const ADMIN_TOKEN: &str = "e2e-admin-token-0123456789abcdef0123456789";

/// What the launcher was handed for one hosted chair.
#[derive(Clone, Debug)]
struct Handed {
    profile: String,
    plays: String,
    name: String,
    fallback: Option<String>,
    listened: bool,
}

/// Every job a launcher was handed, in order.
type Log = Arc<Mutex<Vec<Handed>>>;

/// A launcher that plays no game: it records the job and sits until it is
/// stopped; a check says yes.
struct Recording(Log);

impl Launcher for Recording {
    fn run(
        &self,
        job: SeatJob,
        started: oneshot::Sender<()>,
        stop: oneshot::Receiver<()>,
    ) -> Running<Exit> {
        self.0.lock().unwrap().push(Handed {
            profile: job.profile.clone(),
            plays: job.plays.clone(),
            name: job.name.clone(),
            fallback: job.fallbacks.first().map(|f| f.profile.clone()),
            listened: job.heard.is_some(),
        });
        Box::pin(async move {
            let _ = started.send(());
            let _ = stop.await;
            Exit::Ended(String::new())
        })
    }

    fn probe(&self, _settings: String, _profile: String, _canary: bool) -> Running<Probe> {
        Box::pin(async { Probe::Ok })
    }
}

/// A profile behind an OpenAI-compatible endpoint; two of one vendor may be
/// a profile and its fallback.
fn definition(label: &str, vendor: &str, max_games: u32, fallback: Option<&str>) -> Definition {
    Definition {
        label: label.into(),
        vendor: vendor.into(),
        enabled: true,
        max_games: Some(max_games),
        caps: None,
        canary: false,
        fallback: fallback.map(Into::into),
        profile: json!({"provider": "openai", "model": "model-a",
            "base_url": "https://api.example.invalid/v1"}),
    }
}

fn cli_definition() -> Definition {
    Definition {
        label: "Agent CLI".into(),
        vendor: "Example".into(),
        enabled: true,
        max_games: None,
        caps: None,
        canary: false,
        fallback: None,
        profile: json!({"provider": "cli", "model": "claude"}),
    }
}

/// A seat agent called `name` over TCP, its profiles written before it starts.
fn seat_agent(
    name: &str,
    gateway: String,
    defs: &BTreeMap<String, Definition>,
    log: &Log,
) -> Arc<Host> {
    let dir = std::env::temp_dir().join(format!(
        "baylee-e2e-fallback-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    write_profiles(&dir.join(baylee_seathost::PROFILES_FILE), defs).unwrap();
    Host::new(Config {
        name: name.into(),
        gateway,
        token: SEATHOST_TOKEN.into(),
        capacity: 0,
        state_dir: dir,
        bridge_gateway: None,
        keys: Arc::new(MemoryKeys::default()),
        launcher: Arc::new(Recording(log.clone())),
    })
    .unwrap()
}

/// A console request: status and body.
fn admin(port: u16, method: &str, path: &str, body: &str) -> (u16, String) {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {ADMIN_TOKEN}\r\n\
         X-Baylee-Admin: ada\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let status = raw.split_whitespace().nth(1).unwrap().parse().unwrap();
    let body = raw
        .split_once("\r\n\r\n")
        .map_or("", |(_, b)| b)
        .to_string();
    (status, body)
}

/// `PUT`s `definition` as profile `id` of seat agent `a`.
fn put(admin_port: u16, id: &str, definition: &Definition) -> (u16, String) {
    admin(
        admin_port,
        "PUT",
        &format!("/admin/llm/seathosts/a/profiles/{id}"),
        &serde_json::to_string(definition).unwrap(),
    )
}

/// The `error` sentence of a refusal's body.
fn error_of(body: &str) -> String {
    serde_json::from_str::<Value>(body).unwrap()["error"]
        .as_str()
        .unwrap_or_else(|| panic!("no error in {body}"))
        .to_string()
}

/// The console's row for the seat agent's profile `id`: what the seat agent
/// reported, with the definition's fallback and the profile it diverts to.
fn console_row(admin_port: u16, id: &str) -> Value {
    let (status, body) = admin(admin_port, "GET", "/admin/llm/profiles", "");
    assert_eq!(status, 200, "{body}");
    serde_json::from_str::<Value>(&body).unwrap()["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .and_then(|row| row["hosts"].get(0))
        .cloned()
        .unwrap_or(Value::Null)
}

/// What a player is shown of profile `id`.
fn shown(port: u16, session: &str, id: &str) -> Value {
    let (status, body) = http(port, "GET", "/lobby/llm-profiles", Some(session), "");
    assert_eq!(status, 200, "{body}");
    serde_json::from_str::<Value>(&body).unwrap()["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id)
        .cloned()
        .unwrap_or(Value::Null)
}

async fn until(what: &str, mut done: impl FnMut() -> bool) {
    for _ in 0..common::WAIT_TRIES {
        if done() {
            return;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    panic!("{what} did not happen in time");
}

/// A gateway with a seat agent token and an admin console, and its port.
fn spawn(label: &str) -> (Gateway, u16) {
    let admin_port_file =
        std::env::temp_dir().join(format!("baylee-gw-{label}-{}.admin", std::process::id()));
    let gw = spawn_gateway_with(
        label,
        &[
            ("BAYLEE_SEATHOST_TOKEN", SEATHOST_TOKEN.into()),
            ("BAYLEE_ADMIN_TOKEN", ADMIN_TOKEN.into()),
            ("BAYLEE_ADMIN_BIND", "127.0.0.1:0".into()),
            (
                "BAYLEE_ADMIN_PORT_FILE",
                admin_port_file.display().to_string(),
            ),
        ],
    );
    let admin_port = std::fs::read_to_string(&admin_port_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    (gw, admin_port)
}

/// A room `session` opened, two chairs: the host's chair.
async fn room(lobby: &Lobby, session: &Session) -> baylee_seat::lobby::Chair {
    let deck = Deck::acceptance("Allytifact").unwrap();
    let deck_id = lobby.upload(session, &deck).await.unwrap();
    lobby.open(session, &deck_id, 2, "hosted").await.unwrap()
}

fn order(port: u16, session: &str, game: &str, profile: &str) -> (u16, String) {
    http(
        port,
        "POST",
        &format!("/lobby/games/{game}/chairs/1/hosted"),
        Some(session),
        &format!(r#"{{"profile":"{profile}"}}"#),
    )
}

fn seat_of(port: u16, viewer: &str, game: &str) -> Value {
    let (status, body) = http(port, "GET", "/lobby/games", Some(viewer), "");
    assert_eq!(status, 200, "{body}");
    serde_json::from_str::<Value>(&body).unwrap()["games"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == game)
        .map_or(Value::Null, |g| g["seats"][1].clone())
}

/// The console refuses a fallback that cannot stand by (the profile itself, a
/// name that is no profile's, an agent CLI as the fallback, a vendor other
/// than the profile's), each with the seat agent's own sentence as the body
/// of a `400` and nothing kept; and a valid pair is written and shown with
/// its fallback, as is an agent CLI that names one. (Before, the body was
/// `unknown field fallback`, whatever it said, and a valid pair was refused
/// too.)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_console_refuses_a_fallback_that_cannot_stand_by_and_shows_one_that_can() {
    let (gw, admin_port) = spawn("fallbackconsole");
    let handed_log = Log::default();
    let agent = seat_agent(
        "a",
        format!("http://127.0.0.1:{}", gw.port),
        &BTreeMap::new(),
        &handed_log,
    );
    tokio::spawn(agent.clone().serve());
    until("the seat agent is connected", || {
        admin(admin_port, "GET", "/admin/llm/seathosts", "")
            .1
            .contains(r#""name":"a""#)
    })
    .await;

    // Itself.
    let (status, body) = put(
        admin_port,
        "primary",
        &definition("Model A", "Example", 1, Some("primary")),
    );
    assert_eq!(status, 400, "{body}");
    assert_eq!(error_of(&body), "a profile cannot be its own fallback");
    // A name that is no profile's id.
    let (status, body) = put(
        admin_port,
        "primary",
        &definition("Model A", "Example", 1, Some("a/b")),
    );
    assert_eq!(status, 400, "{body}");
    assert!(
        error_of(&body).contains("a fallback is a profile's id"),
        "{body}"
    );
    // A CLI that names a fallback is kept: a subscription that runs out
    // hands over as an API's quota does.
    let mut cli = cli_definition();
    cli.fallback = Some("backup".into());
    let (status, body) = put(admin_port, "agent-cli", &cli);
    assert_eq!(status, 200, "{body}");
    // A fallback that is a CLI the seat agent holds.
    let (status, body) = put(admin_port, "agent-cli", &cli_definition());
    assert_eq!(status, 200, "{body}");
    let (status, body) = put(
        admin_port,
        "primary",
        &definition("Model A", "Example", 1, Some("agent-cli")),
    );
    assert_eq!(status, 400, "{body}");
    assert!(error_of(&body).contains("plays an agent CLI"), "{body}");
    // A fallback of another vendor.
    let (status, body) = put(
        admin_port,
        "other",
        &definition("Model C", "Other", 1, None),
    );
    assert_eq!(status, 200, "{body}");
    let (status, body) = put(
        admin_port,
        "primary",
        &definition("Model A", "Example", 1, Some("other")),
    );
    assert_eq!(status, 400, "{body}");
    assert!(error_of(&body).contains("their vendors differ"), "{body}");
    // And the same the other way round: a profile that names a fallback not
    // there yet is kept, and the fallback written after it is held to the pair.
    let (status, body) = put(
        admin_port,
        "late-primary",
        &definition("Model D", "Example", 1, Some("late-cli")),
    );
    assert_eq!(status, 200, "{body}");
    let (status, body) = put(admin_port, "late-cli", &cli_definition());
    assert_eq!(status, 400, "{body}");
    assert!(error_of(&body).contains("plays an agent CLI"), "{body}");

    // None of the refused writes was kept.
    assert_eq!(console_row(admin_port, "primary"), Value::Null);
    assert_eq!(console_row(admin_port, "late-cli"), Value::Null);

    // A valid pair: the fallback first, then the profile that names it.
    let (status, body) = put(
        admin_port,
        "backup",
        &definition("Model B", "Example", 1, None),
    );
    assert_eq!(status, 200, "{body}");
    let (status, body) = put(
        admin_port,
        "primary",
        &definition("Model A", "example", 1, Some("backup")),
    );
    assert_eq!(status, 200, "{body}");
    until("the console shows the fallback", || {
        console_row(admin_port, "primary")["fallback"] == "backup"
    })
    .await;
    let primary = console_row(admin_port, "primary");
    assert_eq!(primary["diverts"], Value::Null, "nothing is full");
    assert_eq!(primary["max_games"], 1);
    assert_eq!(console_row(admin_port, "backup")["fallback"], Value::Null);
}

/// A profile at its `max_games` whose fallback is free stays on offer: the
/// console shows it diverting, the player's list shows it available, an order
/// for it is taken and the seat agent's launcher is handed a game that plays
/// the fallback in the chair of the profile ordered, with the room still
/// naming the profile ordered; once the fallback is full as well the order is
/// refused as busy, and when the fallback's game ends the profile diverts
/// again. (Before, an order for a full profile was refused.)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn an_order_for_a_full_profile_is_taken_and_played_on_its_fallback() {
    let (gw, admin_port) = spawn("fallbackorder");
    let (port, http_base) = (gw.port, format!("http://127.0.0.1:{}", gw.port));
    let _agent = attach_agent(&gw).await;
    // Only the profile is ordered, so only it is opened to players.
    let (status, body) = admin(
        admin_port,
        "PUT",
        "/admin/llm/access/primary",
        r#"{"policy":"registered"}"#,
    );
    assert_eq!(status, 200, "{body}");

    let handed_log = Log::default();
    let agent = seat_agent(
        "a",
        http_base.clone(),
        &BTreeMap::from([
            (
                "primary".into(),
                definition("Model A", "Example", 1, Some("backup")),
            ),
            ("backup".into(), definition("Model B", "Example", 1, None)),
        ]),
        &handed_log,
    );
    tokio::spawn(agent.clone().serve());
    // Checked once, now, so that no check comes in the middle of the test.
    for id in ["primary", "backup"] {
        until("the seat agent is connected", || {
            admin(
                admin_port,
                "POST",
                &format!("/admin/llm/seathosts/a/profiles/{id}/probe"),
                "",
            )
            .0 == 202
        })
        .await;
    }
    until("both profiles are checked", || {
        ["primary", "backup"]
            .iter()
            .all(|id| console_row(admin_port, id)["last_ok_unix"].is_i64())
    })
    .await;

    let host = login(port, "hostfb", "TEST-host");
    let lobby = Lobby::new(&http_base);
    let session = Session::from_token(host.clone());

    // The first game plays the profile, the fallback standing by.
    let first = room(&lobby, &session).await;
    let (status, body) = order(port, &host, &first.game_id, "primary");
    assert_eq!(status, 202, "{body}");
    until("the first bridge is started", || {
        handed_log.lock().unwrap().len() == 1
    })
    .await;
    let handed = handed_log.lock().unwrap()[0].clone();
    assert_eq!(
        (handed.profile.as_str(), handed.plays.as_str()),
        ("primary", "primary")
    );
    assert_eq!(handed.fallback.as_deref(), Some("backup"));
    assert!(handed.listened);

    // The profile is full and says its games go on its fallback now.
    until("the console shows the diversion", || {
        console_row(admin_port, "primary")["diverts"] == "backup"
    })
    .await;
    let row = console_row(admin_port, "primary");
    assert_eq!(
        (&row["state"], &row["games"], &row["max_games"]),
        (&json!("available"), &json!(1), &json!(1)),
        "{row}"
    );
    let offered = shown(port, &host, "primary");
    assert_eq!(offered["available"], true, "{offered}");
    assert_eq!(offered["state"], "available", "{offered}");
    assert_eq!(offered["games"], 1, "{offered}");

    // The second order is taken, and plays the fallback in the ordered
    // profile's chair.
    let second = room(&lobby, &session).await;
    let (status, body) = order(port, &host, &second.game_id, "primary");
    assert_eq!(status, 202, "{body}");
    until("the second bridge is started", || {
        handed_log.lock().unwrap().len() == 2
    })
    .await;
    let handed = handed_log.lock().unwrap()[1].clone();
    assert_eq!(
        (handed.profile.as_str(), handed.plays.as_str()),
        ("primary", "backup")
    );
    assert_eq!(
        handed.name, "Model-A",
        "the chair's name is the ordered profile's"
    );
    assert_eq!(handed.fallback, None);
    assert!(!handed.listened);
    let chair = seat_of(port, &host, &second.game_id);
    assert_eq!(chair["hosted"]["profile"], "primary", "{chair}");
    assert_eq!(chair["hosted"]["label"], "Model A", "{chair}");

    // The fallback is full now: the profile no longer diverts and is busy.
    until("the console shows the fallback full", || {
        let (primary, backup) = (
            console_row(admin_port, "primary"),
            console_row(admin_port, "backup"),
        );
        primary["diverts"].is_null() && primary["state"] == "busy" && backup["games"] == 1
    })
    .await;
    let third = room(&lobby, &session).await;
    // The fallback is the operator's choice and plays for anyone who may seat
    // the profile; ordering it itself is governed by its own policy, which
    // here opens it to nobody.
    let (status, body) = order(port, &host, &third.game_id, "backup");
    assert_eq!(status, 403, "{body}");
    let (status, body) = order(port, &host, &third.game_id, "primary");
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("busy"), "{body}");
    assert_eq!(
        handed_log.lock().unwrap().len(),
        2,
        "the refused order started nothing"
    );

    // The host takes the diverted game back: the fallback is free again, and
    // the profile, still full with its own game, diverts again.
    let (status, _) = http(
        port,
        "DELETE",
        &format!("/lobby/games/{}/chairs/1/hosted", second.game_id),
        Some(&host),
        "",
    );
    assert_eq!(status, 204);
    until("the diverted game is released", || {
        let (primary, backup) = (
            console_row(admin_port, "primary"),
            console_row(admin_port, "backup"),
        );
        backup["games"] == 0 && primary["games"] == 1 && primary["diverts"] == "backup"
    })
    .await;
}
