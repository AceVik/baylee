//! Hosted language-model seats (`docs/protocol.md` §"Hosted language-model
//! seats"): two real seat agents (`baylee_seathost::Host`) connected to a
//! real gateway, one on its unix socket and one over TCP, each holding a
//! profile of the same id; players see one row per profile, an exhausted
//! profile listed last; a registered host's orders are spread over the two;
//! a guest is refused; the console writes profiles and a key (write-only,
//! audited, never logged); and a hosted chair plays a whole game.
//!
//! No model is asked and no CLI runs: the seat agents' launcher plays each
//! hosted chair in this process with the house heuristic behind the very
//! lobby and socket code `baylee-seat` runs, after redeeming the gateway's
//! chair ticket as the bridge does. The process path (the real bridge
//! binary and the fake CLI) is `hosted_seat_processes`, ignored.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_client_core::llmseat::keys::{KeyEntry, KeyStore, MemoryKeys};
use baylee_protocol::seathost::Definition;
use baylee_seat::bridge::{self, PlayOptions};
use baylee_seat::deck::Deck;
use baylee_seat::link::SeatLink;
use baylee_seat::lobby::{ChairTicket, Lobby, Session};
use baylee_seat::seat::Outcome;
use baylee_seat::{
    BridgeConfig, Disclosure, HouseMind, Mind, Request, ScriptedMind, SeatCore, Thinking,
    Transcript,
};
use baylee_seathost::launch::{Exit, Launcher, Running, SeatJob};
use baylee_seathost::state::Probe;
use baylee_seathost::{Config, Host, write_profiles};
use common::{Gateway, attach_agent, http, json_field, login, spawn_gateway_with};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

const SEATHOST_TOKEN: &str = "e2e-seathost-token-0123456789abcdef0123";
const ADMIN_TOKEN: &str = "e2e-admin-token-0123456789abcdef0123456789";
const KEY: &str = "sk-ant-api03-hosted-e2e-never-in-a-log-0123456789";

/// A whole game, with room to spare.
const GAME_BUDGET: Duration = Duration::from_secs(600);

/// What the in-process launcher played, by seat agent.
#[derive(Default)]
struct Played {
    started: Mutex<Vec<String>>,
    outcomes: Mutex<Vec<Option<Outcome>>>,
}

/// Plays a hosted chair in this process: redeems the ticket as the bridge
/// does, says ready, and plays the house heuristic to the game's end; or,
/// stopped, gives the chair back.
struct InProcess {
    host: String,
    played: Arc<Played>,
}

impl Launcher for InProcess {
    fn run(
        &self,
        job: SeatJob,
        started: oneshot::Sender<()>,
        stop: oneshot::Receiver<()>,
    ) -> Running<Exit> {
        let played = self.played.clone();
        let host = self.host.clone();
        Box::pin(async move {
            let lobby = Lobby::new(&job.gateway_url);
            let ticket = ChairTicket::read(&job.chair_ticket).expect("a ticket");
            let deck = Deck::acceptance("Victory").unwrap();
            let name = format!("LLM-{}", job.name);
            let (chair, _) = match lobby
                .redeem(&ticket, &job.game_id, job.seat, &name, &deck)
                .await
            {
                Ok(sat) => sat,
                Err(e) => return Exit::Failed(Probe::Failing(format!("{e:#}"))),
            };
            played.started.lock().unwrap().push(host);
            let _ = started.send(());
            let mind: Arc<dyn Mind> = Arc::new(AsModel(HouseMind::default()));
            let play = async {
                lobby.chair_ready(&chair).await?;
                lobby
                    .wait_for_chair_start(&chair, Duration::from_millis(100))
                    .await?;
                let core = SeatCore::new(
                    BridgeConfig::default(),
                    deck.list.clone(),
                    mind.disclosure(),
                );
                let mut link = SeatLink::new(lobby.clone(), chair.clone(), None);
                let options = PlayOptions {
                    min_think: Duration::ZERO,
                    ..PlayOptions::default()
                };
                bridge::play(&mut link, core, mind, &mut Transcript::memory(), &options).await
            };
            tokio::select! {
                done = play => {
                    match done {
                        Ok(game) => {
                            played.outcomes.lock().unwrap().push(game.stats.outcome);
                            Exit::Ended
                        }
                        Err(e) => Exit::Failed(Probe::Failing(format!("{e:#}"))),
                    }
                }
                _ = stop => {
                    let _ = lobby.leave_chair(&chair).await;
                    Exit::Ended
                }
            }
        })
    }

    fn probe(&self, _settings: String, _profile: String, _canary: bool) -> Running<Probe> {
        Box::pin(async { Probe::Ok })
    }
}

/// The house heuristic under a language model's name: a hosted chair is
/// called `LLM-…`, and a bridge plays a chair only under its mind's name.
struct AsModel(HouseMind);

impl Mind for AsModel {
    fn decide(&self, request: Request) -> Thinking<'_> {
        self.0.decide(request)
    }

    fn disclosure(&self) -> Disclosure {
        Disclosure::Llm
    }
}

fn definition(label: &str, max_games: u32, caps: Option<Value>) -> Definition {
    Definition {
        label: label.into(),
        vendor: "Anthropic".into(),
        enabled: true,
        max_games: Some(max_games),
        caps,
        canary: false,
        profile: json!({"provider": "anthropic", "model": "claude-sonnet-5-5"}),
    }
}

/// A seat agent called `name`, its profiles written before it starts.
fn seat_agent(
    name: &str,
    gateway: String,
    bridge_gateway: Option<String>,
    defs: BTreeMap<String, Definition>,
    played: &Arc<Played>,
    keys: Arc<MemoryKeys>,
) -> Arc<Host> {
    let dir = std::env::temp_dir().join(format!(
        "baylee-e2e-seathost-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    write_profiles(&dir.join(baylee_seathost::PROFILES_FILE), &defs).unwrap();
    Host::new(Config {
        name: name.into(),
        gateway,
        token: SEATHOST_TOKEN.into(),
        capacity: 0,
        state_dir: dir,
        bridge_gateway,
        keys,
        launcher: Arc::new(InProcess {
            host: name.into(),
            played: played.clone(),
        }),
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

fn profiles(port: u16, session: &str) -> Vec<Value> {
    let (status, body) = http(port, "GET", "/lobby/llm-profiles", Some(session), "");
    assert_eq!(status, 200, "{body}");
    serde_json::from_str::<Value>(&body).unwrap()["profiles"]
        .as_array()
        .unwrap()
        .clone()
}

fn profile(port: u16, session: &str, id: &str) -> Value {
    profiles(port, session)
        .into_iter()
        .find(|p| p["id"] == id)
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

fn row(port: u16, viewer: &str, game: &str) -> Value {
    let (status, body) = http(port, "GET", "/lobby/games", Some(viewer), "");
    assert_eq!(status, 200, "{body}");
    serde_json::from_str::<Value>(&body).unwrap()["games"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == game)
        .cloned()
        .unwrap_or(Value::Null)
}

/// A room `session` opened (chair 0), two chairs: its id and the host's
/// chair.
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

fn spawn(label: &str, socket: &std::path::Path, admin_port_file: &std::path::Path) -> Gateway {
    spawn_gateway_with(
        label,
        &[
            ("BAYLEE_SEATHOST_TOKEN", SEATHOST_TOKEN.into()),
            ("BAYLEE_UNIX_SOCKET", socket.display().to_string()),
            ("BAYLEE_ADMIN_TOKEN", ADMIN_TOKEN.into()),
            ("BAYLEE_ADMIN_BIND", "127.0.0.1:0".into()),
            (
                "BAYLEE_ADMIN_PORT_FILE",
                admin_port_file.display().to_string(),
            ),
            ("RUST_LOG", "debug".into()),
        ],
    )
}

// Many threads: the console and lobby calls below block, and the seat
// agents in this process must answer meanwhile.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn hosted_profiles_are_aggregated_balanced_guarded_and_play_a_whole_game() {
    let id = std::process::id();
    let socket = std::env::temp_dir().join(format!("baylee-gw-hosted-{id}.sock"));
    let admin_port_file = std::env::temp_dir().join(format!("baylee-gw-hosted-{id}.admin"));
    let gw = spawn("hosted", &socket, &admin_port_file);
    let port = gw.port;
    let admin_port: u16 = std::fs::read_to_string(&admin_port_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let _agent = attach_agent(&gw).await;
    let http_base = format!("http://127.0.0.1:{port}");

    // Seat agent "a" on the unix socket holds sonnet and opus, opus with a
    // cap of one dollar a day and a book that spent two; "b" over TCP holds
    // sonnet only. Each sonnet plays one game at a time.
    let played = Arc::new(Played::default());
    let keys_a = Arc::new(MemoryKeys::default());
    let a = seat_agent(
        "a",
        format!("unix:{}", socket.display()),
        Some(http_base.clone()),
        BTreeMap::from([
            ("sonnet".into(), definition("Sonnet", 1, None)),
            (
                "opus".into(),
                definition("Opus", 1, Some(json!({"day_usd": 1.0}))),
            ),
        ]),
        &played,
        keys_a.clone(),
    );
    let book = a.state_dir().join("spend").join("opus.json");
    std::fs::create_dir_all(book.parent().unwrap()).unwrap();
    let now = baylee_seathost::now();
    let day = baylee_client_core::llmseat::ledger::Moment {
        unix: now,
        offset: None,
    }
    .day();
    std::fs::write(
        &book,
        json!({"version": 1, "games": [{"id": 1, "at": now, "day": day, "model": "claude-opus-5-5",
            "reserved_usd": 2.0, "spent_usd": 2.0, "settled": now}]})
        .to_string(),
    )
    .unwrap();
    let b = seat_agent(
        "b",
        http_base.clone(),
        None,
        BTreeMap::from([("sonnet".into(), definition("Sonnet", 1, None))]),
        &played,
        Arc::new(MemoryKeys::default()),
    );
    tokio::spawn(a.clone().serve());
    tokio::spawn(b.clone().serve());

    // Named as the scripted mind that plays its chair must be (`TEST-`).
    let host = login(port, "hosta", "TEST-host");
    until("both seat agents report", || {
        let rows = profiles(port, &host);
        rows.len() == 2 && rows[0]["max_games"] == 2 && rows[0]["state"] == "available"
    })
    .await;
    let rows = profiles(port, &host);
    assert_eq!(rows[0]["id"], "sonnet", "{rows:?}");
    assert_eq!(rows[0]["available"], true);
    assert_eq!(rows[1]["id"], "opus", "the exhausted one is listed, last");
    assert_eq!(rows[1]["state"], "exhausted");
    assert_eq!(rows[1]["available"], false);
    assert!(rows[1]["until_unix"].as_i64().unwrap() > now, "{rows:?}");
    let (status, body) = http(port, "GET", "/info", None, "");
    assert_eq!(status, 200);
    assert!(body.contains("\"hosted_llm\":true"), "{body}");

    let lobby = Lobby::new(&http_base);
    let session = Session::from_token(host.clone());
    let first = room(&lobby, &session).await;

    // A guest may list the models and may not seat one.
    let (status, body) = http(
        port,
        "POST",
        "/auth/guest",
        None,
        r#"{"display_name":"Visitor"}"#,
    );
    assert_eq!(status, 200, "{body}");
    let guest = json_field(&body, "token").to_string();
    assert_eq!(profiles(port, &guest).len(), 2);
    let guest_room = room(&lobby, &Session::from_token(guest.clone())).await;
    let (status, body) = order(port, &guest, &guest_room.game_id, "sonnet");
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("guest"), "{body}");
    // Nor in someone else's room.
    let (status, _) = order(port, &guest, &first.game_id, "sonnet");
    assert_eq!(status, 403);
    // An exhausted profile is not offered for seating.
    let (status, body) = order(port, &host, &first.game_id, "opus");
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("exhausted"), "{body}");
    assert_eq!(order(port, &host, &first.game_id, "nothing").0, 409);

    // Two orders of sonnet go to the two seat agents; a third finds both
    // at their bound.
    let (status, body) = order(port, &host, &first.game_id, "sonnet");
    assert_eq!(status, 202, "{body}");
    let listed = row(port, &host, &first.game_id);
    assert_eq!(listed["seats"][1]["taken"], true, "{listed}");
    assert_eq!(
        listed["seats"][1]["hosted"]["vendor"], "Anthropic",
        "{listed}"
    );
    let second = room(&lobby, &session).await;
    let (status, body) = order(port, &host, &second.game_id, "sonnet");
    assert_eq!(status, 202, "{body}");
    until("both bridges sit", || {
        played.started.lock().unwrap().len() == 2
    })
    .await;
    let mut hosts = played.started.lock().unwrap().clone();
    hosts.sort();
    assert_eq!(hosts, ["a", "b"], "one order each");
    until("the reports say busy", || {
        profile(port, &host, "sonnet")["games"] == 2
    })
    .await;
    let third = room(&lobby, &session).await;
    let (status, body) = order(port, &host, &third.game_id, "sonnet");
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("busy"), "{body}");
    assert_eq!(profile(port, &host, "sonnet")["state"], "busy");
    // Neither is available now: both listed, by label.
    let ids: Vec<Value> = profiles(port, &host)
        .iter()
        .map(|p| p["id"].clone())
        .collect();
    assert_eq!(ids, [json!("opus"), json!("sonnet")]);

    // The host takes the second room's model back.
    let (status, _) = http(
        port,
        "DELETE",
        &format!("/lobby/games/{}/chairs/1/hosted", second.game_id),
        Some(&host),
        "",
    );
    assert_eq!(status, 204);
    until("the second bridge is stopped", || {
        profile(port, &host, "sonnet")["games"] == 1
    })
    .await;

    // The first room plays a whole game: the host's chair plays nothing,
    // the hosted chair (the house, standing in for a model) wins.
    until("the hosted chair is ready", || {
        row(port, &host, &first.game_id)["seats"][1]["hosted"]["state"] == "ready"
    })
    .await;
    lobby.ready(&session, &first.game_id).await.unwrap();
    lobby.start(&session, &first.game_id).await.unwrap();
    let scripted: Arc<dyn Mind> = Arc::new(ScriptedMind::idle());
    let core = SeatCore::new(
        BridgeConfig::default(),
        Deck::acceptance("Allytifact").unwrap().list,
        scripted.disclosure(),
    );
    let mut link = SeatLink::new(lobby.clone(), first.clone(), Some(session.clone()));
    let options = PlayOptions {
        min_think: Duration::ZERO,
        ..PlayOptions::default()
    };
    let mine = tokio::time::timeout(
        GAME_BUDGET,
        bridge::play(
            &mut link,
            core,
            scripted,
            &mut Transcript::memory(),
            &options,
        ),
    )
    .await
    .expect("the game ends in time")
    .expect("the host's chair plays");
    assert_eq!(mine.stats.outcome, Some(Outcome::Lost));
    until("the hosted bridge ends its game", || {
        !played.outcomes.lock().unwrap().is_empty()
    })
    .await;
    assert_eq!(
        played.outcomes.lock().unwrap().as_slice(),
        [Some(Outcome::Won)]
    );
    until("the seat agents report no game", || {
        profile(port, &host, "sonnet")["games"] == 0
    })
    .await;
    let host_id = {
        let (_, body) = http(port, "GET", "/me", Some(&host), "");
        json_field(&body, "id").to_string()
    };
    until("the record is stored", || {
        gw.scalar(&format!(
            "SELECT count(*) FROM game_record_seat WHERE game_id = '{}'",
            first.game_id
        )) == 2
    })
    .await;
    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM game_record_seat WHERE game_id = '{}' AND seat = 1 \
             AND account_id IS NULL AND delegated_by = '{host_id}'",
            first.game_id
        )),
        1,
        "the hosted chair is the host's delegate"
    );

    // The console: both seat agents, one row per profile with its hosts.
    let (status, body) = admin(admin_port, "GET", "/admin/llm/seathosts", "");
    assert_eq!(status, 200, "{body}");
    let seathosts: Value = serde_json::from_str(&body).unwrap();
    let names: Vec<&str> = seathosts["seathosts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["a", "b"]);
    assert_eq!(seathosts["seathosts"][0]["local"], true);
    assert_eq!(seathosts["seathosts"][1]["local"], false);
    let (status, body) = admin(admin_port, "GET", "/admin/llm/profiles", "");
    assert_eq!(status, 200, "{body}");
    let console: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(console["profiles"][0]["hosts"].as_array().unwrap().len(), 2);

    // A profile written on "b" through the console, then refused one.
    let (status, body) = admin(
        admin_port,
        "PUT",
        "/admin/llm/seathosts/b/profiles/haiku",
        &serde_json::to_string(&definition("Haiku", 2, None)).unwrap(),
    );
    assert_eq!(status, 200, "{body}");
    until("b reports haiku", || {
        profiles(port, &host).iter().any(|p| p["id"] == "haiku")
    })
    .await;
    let mut keyed = definition("Bad", 1, None);
    keyed.profile = json!({"provider": "anthropic", "model": KEY});
    let (status, body) = admin(
        admin_port,
        "PUT",
        "/admin/llm/seathosts/b/profiles/bad",
        &serde_json::to_string(&keyed).unwrap(),
    );
    assert_eq!(status, 400, "{body}");
    assert!(!body.contains("never-in-a-log"), "{body}");
    let (status, _) = admin(
        admin_port,
        "POST",
        "/admin/llm/seathosts/b/profiles/haiku/enabled",
        r#"{"enabled":false}"#,
    );
    assert_eq!(status, 204);
    until("haiku reads disabled", || {
        profiles(port, &host)
            .iter()
            .any(|p| p["id"] == "haiku" && p["state"] == "disabled")
    })
    .await;
    assert_eq!(
        admin(
            admin_port,
            "POST",
            "/admin/llm/seathosts/a/profiles/sonnet/probe",
            ""
        )
        .0,
        202
    );
    assert_eq!(
        admin(
            admin_port,
            "DELETE",
            "/admin/llm/seathosts/b/profiles/haiku",
            ""
        )
        .0,
        204
    );
    assert_eq!(
        admin(
            admin_port,
            "POST",
            "/admin/llm/seathosts/zz/profiles/sonnet/probe",
            ""
        )
        .0,
        404
    );

    // A key: only to the seat agent on the unix socket, write-only.
    let key_body = format!(r#"{{"key":"{KEY}"}}"#);
    let (status, body) = admin(
        admin_port,
        "POST",
        "/admin/llm/seathosts/b/profiles/sonnet/key",
        &key_body,
    );
    assert_eq!(status, 503, "{body}");
    assert!(body.contains("unix socket"), "{body}");
    let (status, body) = admin(
        admin_port,
        "POST",
        "/admin/llm/seathosts/a/profiles/sonnet/key",
        &key_body,
    );
    assert_eq!(status, 204, "{body}");
    let entry = KeyEntry::named("ANTHROPIC_API_KEY", "api.anthropic.com").unwrap();
    assert_eq!(keys_a.get(&entry).unwrap().as_deref(), Some(KEY));
    until("a reports the key kept", || {
        let (_, body) = admin(admin_port, "GET", "/admin/llm/seathosts", "");
        body.contains("\"key\":\"kept\"")
    })
    .await;
    for path in ["/admin/llm/seathosts", "/admin/llm/profiles"] {
        let (_, body) = admin(admin_port, "GET", path, "");
        assert!(!body.contains("never-in-a-log"), "{path}: {body}");
    }

    // Audited, and never the key, even at debug.
    let logs = gw.logs();
    for action in [
        "llm.profile.write",
        "llm.profile.disable",
        "llm.profile.probe",
        "llm.profile.delete",
        "llm.key.set",
    ] {
        let line = logs
            .lines()
            .find(|l| l.contains(action) && l.contains("baylee_gateway::audit"))
            .unwrap_or_else(|| panic!("no audit line for {action}"));
        assert!(line.contains("ada"), "{line}");
    }
    assert!(
        !logs.contains("never-in-a-log"),
        "the key reached the gateway's log"
    );
}
