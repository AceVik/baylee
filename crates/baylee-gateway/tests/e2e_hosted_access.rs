//! Who may seat which hosted language model (owner, 13.10.2026;
//! `docs/protocol.md` §"Who may seat which model"): a real gateway with a
//! real seat agent (`baylee_seathost::Host`, its bridges played in process)
//! and the admin console, asked the way the feedback service asks it.
//!
//! What they hold: a profile with no policy is open to nobody; `registered`
//! opens it to every registered account and never to a guest; `listed` to the
//! accounts on its list alone; `nobody` hides it again; the check happens at
//! the order only, so a chair already ordered outlives a later change; one
//! profile at most is the default for registered accounts; a policy may be
//! set for an id no seat agent reports; an allow is all or nothing, refuses
//! a guest's account and an unknown one; deleting an account takes it off
//! every list; the type-ahead finds registered accounts by prefix, ten at
//! most, `%` and `_` literal; and each change is audited by id, never by name.

#![allow(clippy::missing_docs_in_private_items)]
// Every scenario needs a Unix-domain seat-host socket; its helpers share that scope.
#![cfg(unix)]

mod common;

use baylee_client_core::llmseat::keys::MemoryKeys;
use baylee_protocol::seathost::Definition;
use baylee_seat::deck::Deck;
use baylee_seat::lobby::{ChairTicket, Lobby, Session};
use baylee_seathost::launch::{Exit, Launcher, Running, SeatJob};
use baylee_seathost::state::Probe;
use baylee_seathost::{Config, Host, write_profiles};
use common::{Gateway, attach_agent, http, json_field, login, spawn_gateway_with};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::oneshot;

const SEATHOST_TOKEN: &str = "e2e-seathost-token-0123456789abcdef0123";
const ADMIN_TOKEN: &str = "e2e-admin-token-0123456789abcdef0123456789";

/// What the gateway says to an order it will not take for lack of a place on
/// the profile's list.
const NOT_OPEN: &str = "that hosted model is not open to your account";

/// Sits a hosted chair the way a bridge does (redeems the gateway's ticket,
/// says ready) and keeps it until told to stop. It plays nothing: these
/// tests are about who may order, not about a game.
struct Sits;

impl Launcher for Sits {
    fn run(
        &self,
        job: SeatJob,
        started: oneshot::Sender<()>,
        stop: oneshot::Receiver<()>,
    ) -> Running<Exit> {
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
            let _ = started.send(());
            if let Err(e) = lobby.chair_ready(&chair).await {
                return Exit::Failed(Probe::Failing(format!("{e:#}")));
            }
            let _ = stop.await;
            let _ = lobby.leave_chair(&chair).await;
            Exit::Ended(String::new())
        })
    }

    fn probe(&self, _settings: String, _profile: String, _canary: bool) -> Running<Probe> {
        Box::pin(async { Probe::Ok })
    }
}

fn definition(label: &str) -> Definition {
    Definition {
        label: label.into(),
        vendor: "Anthropic".into(),
        enabled: true,
        max_games: Some(8),
        caps: None,
        canary: false,
        fallback: None,
        profile: json!({"provider": "anthropic", "model": "claude-sonnet-5-5"}),
    }
}

/// A gateway, its console and one seat agent holding `ids` (labelled by
/// their id with a capital).
struct Rig {
    gw: Gateway,
    port: u16,
    admin_port: u16,
    http_base: String,
    _agent: tokio::task::JoinHandle<()>,
    _seat_agent: tokio::task::JoinHandle<()>,
}

async fn rig(label: &str, ids: &[&str]) -> Rig {
    let pid = std::process::id();
    let socket = std::env::temp_dir().join(format!("baylee-gw-{label}-{pid}.sock"));
    let admin_port_file = std::env::temp_dir().join(format!("baylee-gw-{label}-{pid}.admin"));
    let gw = spawn_gateway_with(
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
    );
    let port = gw.port;
    let admin_port: u16 = std::fs::read_to_string(&admin_port_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let agent = attach_agent(&gw).await;
    let http_base = format!("http://127.0.0.1:{port}");

    let dir = std::env::temp_dir().join(format!(
        "baylee-e2e-access-{label}-{pid}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let defs: BTreeMap<String, Definition> = ids
        .iter()
        .map(|id| {
            let mut label = (*id).to_string();
            label[..1].make_ascii_uppercase();
            ((*id).to_string(), definition(&label))
        })
        .collect();
    write_profiles(&dir.join(baylee_seathost::PROFILES_FILE), &defs).unwrap();
    let seat_agent = Host::new(Config {
        name: "a".into(),
        gateway: format!("unix:{}", socket.display()),
        token: SEATHOST_TOKEN.into(),
        capacity: 0,
        state_dir: dir,
        bridge_gateway: Some(http_base.clone()),
        keys: Arc::new(MemoryKeys::default()),
        launcher: Arc::new(Sits),
    })
    .unwrap();
    let seat_agent = tokio::spawn(seat_agent.serve());
    let rig = Rig {
        gw,
        port,
        admin_port,
        http_base,
        _agent: agent,
        _seat_agent: seat_agent,
    };
    until("the seat agent reports its profiles", || {
        let (status, body) = rig.console("GET", "/admin/llm/access", "");
        status == 200
            && ids.iter().all(|id| {
                body["profiles"].as_array().is_some_and(|rows| {
                    rows.iter()
                        .any(|p| p["id"] == *id && p["connected"] == true)
                })
            })
    })
    .await;
    rig
}

impl Rig {
    /// A console request: status and the body, as JSON when it is.
    fn console(&self, method: &str, path: &str, body: &str) -> (u16, Value) {
        use std::io::{Read, Write};
        let port = self.admin_port;
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
        let body = raw.split_once("\r\n\r\n").map_or("", |(_, b)| b);
        let json = serde_json::from_str(body).unwrap_or_else(|_| Value::String(body.into()));
        (status, json)
    }

    /// The console answers `status`; its body, for the assertions after.
    fn expect(&self, status: u16, method: &str, path: &str, body: &str) -> Value {
        let (got, json) = self.console(method, path, body);
        assert_eq!(got, status, "{method} {path} {body}: {json}");
        json
    }

    #[allow(clippy::needless_pass_by_value)] // a `json!` at every call
    fn put(&self, id: &str, body: Value) -> Value {
        self.expect(
            200,
            "PUT",
            &format!("/admin/llm/access/{id}"),
            &body.to_string(),
        )
    }

    fn detail(&self, id: &str) -> Value {
        self.expect(200, "GET", &format!("/admin/llm/access/{id}"), "")
    }

    /// The profile's row in the console's list.
    fn row(&self, id: &str) -> Value {
        self.expect(200, "GET", "/admin/llm/access", "")["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == id)
            .cloned()
            .unwrap_or(Value::Null)
    }

    fn allow(&self, id: &str, accounts: &[&str]) -> (u16, Value) {
        self.console(
            "POST",
            &format!("/admin/llm/access/{id}/allow"),
            &json!({ "accounts": accounts }).to_string(),
        )
    }

    fn disallow(&self, id: &str, accounts: &[&str]) -> (u16, Value) {
        self.console(
            "POST",
            &format!("/admin/llm/access/{id}/disallow"),
            &json!({ "accounts": accounts }).to_string(),
        )
    }

    /// The ids of the profiles `session` is listed, sorted.
    fn seen_by(&self, session: &str) -> Vec<String> {
        let (status, body) = http(self.port, "GET", "/lobby/llm-profiles", Some(session), "");
        assert_eq!(status, 200, "{body}");
        let mut ids: Vec<String> = serde_json::from_str::<Value>(&body).unwrap()["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["id"].as_str().unwrap().to_string())
            .collect();
        ids.sort();
        ids
    }

    fn order(&self, session: &str, game: &str, profile: &str) -> (u16, String) {
        http(
            self.port,
            "POST",
            &format!("/lobby/games/{game}/chairs/1/hosted"),
            Some(session),
            &json!({ "profile": profile }).to_string(),
        )
    }

    /// A room `session` opened: its game id.
    async fn room(&self, session: &str) -> String {
        let lobby = Lobby::new(&self.http_base);
        let session = Session::from_token(session.to_string());
        let deck = Deck::acceptance("Allytifact").unwrap();
        let deck_id = lobby.upload(&session, &deck).await.unwrap();
        lobby
            .open(&session, &deck_id, 2, "hosted")
            .await
            .unwrap()
            .game_id
    }

    /// The listing's row for `game`.
    fn listed(&self, viewer: &str, game: &str) -> Value {
        let (status, body) = http(self.port, "GET", "/lobby/games", Some(viewer), "");
        assert_eq!(status, 200, "{body}");
        serde_json::from_str::<Value>(&body).unwrap()["games"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["id"] == game)
            .cloned()
            .unwrap_or(Value::Null)
    }

    fn account_id(&self, session: &str) -> String {
        let (status, body) = http(self.port, "GET", "/me", Some(session), "");
        assert_eq!(status, 200, "{body}");
        json_field(&body, "id").to_string()
    }

    fn guest(&self, name: &str) -> String {
        let (status, body) = http(
            self.port,
            "POST",
            "/auth/guest",
            None,
            &json!({ "display_name": name }).to_string(),
        );
        assert_eq!(status, 200, "{body}");
        json_field(&body, "token").to_string()
    }
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

/// A list of this one account.
fn only(id: &str) -> Vec<String> {
    vec![id.to_string()]
}

/// The ids on a detail's list.
fn listed_ids(detail: &Value) -> Vec<String> {
    detail["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap().to_string())
        .collect()
}

// Many threads: the console and lobby calls below block, and the seat agent
// in this process must answer meanwhile.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn a_policy_decides_who_is_listed_a_model_and_who_may_order_it() {
    let rig = rig("acc1", &["alpha", "beta"]).await;
    let ann = login(rig.port, "annalise", "Annalise");
    let bob = login(rig.port, "bobby", "Bobby");
    let guest = rig.guest("Visitor");
    let ann_id = rig.account_id(&ann);
    let bob_id = rig.account_id(&bob);
    let ann_room = rig.room(&ann).await;
    let ann_second_room = rig.room(&ann).await;
    let bob_room = rig.room(&bob).await;
    let guest_room = rig.room(&guest).await;

    // (a) A new profile is open to nobody: listed in the console as `listed`
    // with an empty list, never stored, and no player sees it.
    for id in ["alpha", "beta"] {
        let row = rig.row(id);
        assert_eq!(row["policy"], "listed", "{row}");
        assert_eq!(row["stored"], false, "{row}");
        assert_eq!(row["listed"], 0, "{row}");
        assert_eq!(row["default_for_registered"], false, "{row}");
        assert_eq!(row["connected"], true, "{row}");
        assert_eq!(row["vendor"], "Anthropic", "{row}");
    }
    assert_eq!(rig.row("alpha")["label"], "Alpha");
    assert!(rig.seen_by(&ann).is_empty());
    assert!(rig.seen_by(&bob).is_empty());
    let (status, body) = rig.order(&ann, &ann_room, "alpha");
    assert_eq!(status, 403, "{body}");
    assert!(body.contains(NOT_OPEN), "{body}");
    let (status, body) = rig.order(&ann, &ann_room, "nothing-like-it");
    assert_eq!(status, 403, "a profile nobody has: {body}");
    assert!(body.contains(NOT_OPEN), "{body}");

    // (b) Registered: every registered account sees alpha, and beta stays
    // hidden. A guest sees nothing and is refused as a guest, the older
    // refusal, not as one off a list.
    let done = rig.put("alpha", json!({ "policy": "registered" }));
    assert_eq!(done["policy"], "registered", "{done}");
    assert_eq!(done["stored"], true, "{done}");
    assert_eq!(done["updated_by"], "ada", "{done}");
    assert_eq!(rig.seen_by(&ann), ["alpha"]);
    assert_eq!(rig.seen_by(&bob), ["alpha"]);
    assert!(
        rig.seen_by(&guest).is_empty(),
        "a guest is admitted to none"
    );
    for profile in ["alpha", "beta"] {
        let (status, body) = rig.order(&guest, &guest_room, profile);
        assert_eq!(status, 403, "{body}");
        assert!(body.contains("guest"), "{profile}: {body}");
        assert!(
            !body.contains(NOT_OPEN),
            "the guest refusal comes first: {body}"
        );
    }
    let (status, body) = rig.order(&bob, &bob_room, "beta");
    assert_eq!(status, 403, "{body}");
    assert!(body.contains(NOT_OPEN), "{body}");

    // (c) Listed: only the accounts on the list. Putting ann on it creates
    // nothing for bob.
    let done = rig.put("beta", json!({ "policy": "listed" }));
    assert_eq!(done["policy"], "listed");
    let (status, done) = rig.allow("beta", &[&ann_id]);
    assert_eq!(status, 200, "{done}");
    assert_eq!(done["added"], 1, "{done}");
    assert_eq!(listed_ids(&done), only(&ann_id));
    assert_eq!(done["listed"], 1, "{done}");
    assert_eq!(done["accounts"][0]["username"], "annalise");
    assert_eq!(done["accounts"][0]["display_name"], "Annalise");
    assert!(
        done["accounts"][0]["handle"]
            .as_str()
            .unwrap()
            .starts_with("Annalise#"),
        "{done}"
    );
    assert_eq!(done["accounts"][0]["added_by"], "ada", "{done}");
    assert_eq!(rig.seen_by(&ann), ["alpha", "beta"]);
    assert_eq!(rig.seen_by(&bob), ["alpha"]);
    let (status, body) = rig.order(&bob, &bob_room, "beta");
    assert_eq!(status, 403, "{body}");
    assert!(body.contains(NOT_OPEN), "{body}");
    let (status, body) = rig.order(&ann, &ann_room, "beta");
    assert_eq!(status, 202, "{body}");

    // (d) Nobody takes it off the table again, and the list is kept for the
    // day it is listed again. The order placed a moment ago still stands: it
    // is checked at the order only, so the chair goes on to say ready and
    // the room's listing keeps naming its model.
    let done = rig.put("beta", json!({ "policy": "nobody" }));
    assert_eq!(done["policy"], "nobody", "{done}");
    assert_eq!(rig.seen_by(&ann), ["alpha"]);
    let (status, body) = rig.order(&ann, &ann_second_room, "beta");
    assert_eq!(status, 403, "a listed account is no more admitted: {body}");
    assert!(body.contains(NOT_OPEN), "{body}");
    until("the chair ordered before the change is ready", || {
        let chair = &rig.listed(&ann, &ann_room)["seats"][1]["hosted"];
        assert_ne!(chair["state"], "failed", "{chair}");
        chair["profile"] == "beta" && chair["state"] == "ready"
    })
    .await;
    let done = rig.put("beta", json!({ "policy": "listed" }));
    assert_eq!(listed_ids(&done), only(&ann_id), "the list was kept");
    assert_eq!(rig.seen_by(&ann), ["alpha", "beta"]);
    assert_eq!(rig.seen_by(&bob), ["alpha"]);

    // (i) The same for a model open to everyone: ordered, then closed.
    let (status, body) = rig.order(&ann, &ann_second_room, "alpha");
    assert_eq!(status, 202, "{body}");
    rig.put("alpha", json!({ "policy": "nobody" }));
    assert_eq!(rig.seen_by(&ann), ["beta"]);
    assert!(rig.seen_by(&bob).is_empty());
    until("the second chair keeps its model and says ready", || {
        let chair = &rig.listed(&ann, &ann_second_room)["seats"][1]["hosted"];
        assert_ne!(chair["state"], "failed", "{chair}");
        chair["profile"] == "alpha" && chair["state"] == "ready"
    })
    .await;
    let listed = rig.listed(&bob, &ann_second_room);
    assert_eq!(listed["seats"][1]["taken"], true, "{listed}");
    assert_eq!(
        listed["seats"][1]["hosted"]["vendor"], "Anthropic",
        "{listed}"
    );
    // Taking the list away from ann changes what she may order next, not
    // the chair she holds.
    let (status, done) = rig.disallow("beta", &[&ann_id]);
    assert_eq!(status, 200, "{done}");
    assert_eq!(done["removed"], 1, "{done}");
    assert!(rig.seen_by(&ann).is_empty());
    assert_eq!(
        rig.listed(&ann, &ann_room)["seats"][1]["hosted"]["profile"],
        "beta"
    );
    // And another account put on the list is admitted at once.
    let (status, done) = rig.allow("beta", &[&bob_id]);
    assert_eq!(status, 200, "{done}");
    assert_eq!(rig.seen_by(&bob), ["beta"]);
    let (status, body) = rig.order(&bob, &bob_room, "beta");
    assert_eq!(status, 202, "{body}");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn the_console_sets_policies_lists_accounts_and_audits_by_id() {
    let rig = rig("acc2", &["alpha", "beta"]).await;

    // (e) One default at most, and only for a profile open to every
    // registered account.
    let done = rig.put(
        "alpha",
        json!({ "policy": "registered", "default_for_registered": true }),
    );
    assert_eq!(done["default_for_registered"], true, "{done}");
    assert_eq!(rig.row("alpha")["default_for_registered"], true);
    assert_eq!(rig.row("beta")["default_for_registered"], false);
    rig.put(
        "beta",
        json!({ "policy": "registered", "default_for_registered": true }),
    );
    assert_eq!(rig.row("alpha")["default_for_registered"], false, "cleared");
    assert_eq!(rig.row("alpha")["policy"], "registered", "only the flag");
    assert_eq!(rig.row("beta")["default_for_registered"], true);
    let refused = rig.expect(
        400,
        "PUT",
        "/admin/llm/access/alpha",
        r#"{"policy":"listed","default_for_registered":true}"#,
    );
    assert!(refused.to_string().contains("registered"), "{refused}");
    rig.expect(
        400,
        "PUT",
        "/admin/llm/access/alpha",
        r#"{"policy":"nobody","default_for_registered":true}"#,
    );
    assert_eq!(
        rig.row("alpha")["policy"],
        "registered",
        "unchanged by a 400"
    );
    assert_eq!(rig.row("alpha")["default_for_registered"], false);
    assert_eq!(rig.row("beta")["default_for_registered"], true);
    // A non-registered policy takes the flag away.
    let done = rig.put("beta", json!({ "policy": "listed" }));
    assert_eq!(done["default_for_registered"], false, "{done}");
    assert_eq!(rig.row("beta")["default_for_registered"], false);
    rig.put(
        "beta",
        json!({ "policy": "registered", "default_for_registered": true }),
    );
    rig.put("beta", json!({ "policy": "nobody" }));
    assert_eq!(rig.row("beta")["default_for_registered"], false);
    rig.put("beta", json!({ "policy": "registered" }));
    assert_eq!(
        rig.row("beta")["default_for_registered"],
        false,
        "stays off"
    );
    let done = rig.put(
        "beta",
        json!({ "policy": "registered", "default_for_registered": false }),
    );
    assert_eq!(done["default_for_registered"], false);

    // What a policy order may not say.
    for body in [
        r#"{"policy":"everyone"}"#,
        r#"{"policy":""}"#,
        r#"{"policy":"Registered"}"#,
        r#"{"default_for_registered":true}"#,
        r#"{"policy":"listed","extra":1}"#,
        "not json",
        "",
    ] {
        rig.expect(400, "PUT", "/admin/llm/access/alpha", body);
    }
    let long = "a".repeat(65);
    for id in ["bad%20id", "bad%40id", long.as_str()] {
        rig.expect(
            400,
            "PUT",
            &format!("/admin/llm/access/{id}"),
            r#"{"policy":"listed"}"#,
        );
        rig.expect(400, "GET", &format!("/admin/llm/access/{id}"), "");
        rig.expect(
            400,
            "POST",
            &format!("/admin/llm/access/{id}/allow"),
            r#"{"accounts":["0199aaaa-0000-7000-8000-000000000001"]}"#,
        );
    }
    rig.put(&"a".repeat(64), json!({ "policy": "listed" }));
    rig.put("Sonnet-5.5_x", json!({ "policy": "listed" }));

    // (f) A policy for an id no seat agent reports.
    let done = rig.put("gamma", json!({ "policy": "listed" }));
    assert_eq!(done["policy"], "listed", "{done}");
    assert_eq!(done["stored"], true, "{done}");
    assert_eq!(done["connected"], false, "{done}");
    assert_eq!(done["label"], Value::Null, "{done}");
    let row = rig.row("gamma");
    assert_eq!(row["connected"], false, "{row}");
    assert_eq!(row["stored"], true, "{row}");
    assert_eq!(rig.row("alpha")["connected"], true);
    // A stored policy and a reported profile are one list, by id.
    let ids: Vec<String> = rig.expect(200, "GET", "/admin/llm/access", "")["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        ["alpha", "beta", "gamma"]
            .iter()
            .all(|id| ids.contains(&(*id).to_string())),
        "{ids:?}"
    );
    rig.put("gamma", json!({ "policy": "nobody" }));
    assert_eq!(rig.row("gamma")["policy"], "nobody");
    let detail = rig.detail("gamma");
    assert_eq!(detail["accounts"], json!([]), "{detail}");
    // An id never heard of reads as listed to nobody.
    let unknown = rig.detail("never-heard-of");
    assert_eq!(unknown["policy"], "listed", "{unknown}");
    assert_eq!(unknown["stored"], false, "{unknown}");
    assert_eq!(unknown["accounts"], json!([]), "{unknown}");

    // (g) Lists. Accounts the lookup tests use are put into the table, as
    // registering a dozen would run into the sign-up allowance.
    rig.gw.sql(
        "INSERT INTO account (id, display_name, username, username_key, lang, created_at, password_hash) \
         SELECT uuidv7(), 'Pfx' || lpad(n::text, 2, '0'), 'pfx' || lpad(n::text, 2, '0'), \
                'pfx' || lpad(n::text, 2, '0'), 'en', now(), 'not a hash' \
         FROM generate_series(1, 12) n",
    );
    rig.gw.sql(
        "INSERT INTO account (id, display_name, username, username_key, lang, created_at, password_hash) \
         VALUES (uuidv7(), 'Mirabel', 'quux77', 'quux77', 'en', now(), 'not a hash'), \
                (uuidv7(), 'Abxcd', 'abxcd', 'abxcd', 'en', now(), 'not a hash'), \
                (uuidv7(), 'Abunder', 'ab_cd', 'ab_cd', 'en', now(), 'not a hash')",
    );
    let carla = login(rig.port, "carla", "Carla");
    let carla_id = rig.account_id(&carla);
    let dora = login(rig.port, "dora", "Dora");
    let dora_id = rig.account_id(&dora);
    let guest = rig.guest("Pfxvisitor");
    let guest_id = rig.account_id(&guest);
    let other = rig
        .gw
        .text("SELECT id::text FROM account WHERE username = 'pfx01'");
    let nobody = "0199aaaa-0000-7000-8000-000000000001";

    // The first allow makes the row, as listed.
    assert_eq!(
        rig.row("delta"),
        Value::Null,
        "no row before the first allow"
    );
    let (status, done) = rig.allow("delta", &[&carla_id]);
    assert_eq!(status, 200, "{done}");
    assert_eq!(done["added"], 1, "{done}");
    assert_eq!(done["policy"], "listed", "{done}");
    assert_eq!(done["stored"], true, "{done}");
    // Again: it is on the list already.
    let (status, done) = rig.allow("delta", &[&carla_id]);
    assert_eq!(status, 200, "{done}");
    assert_eq!(done["added"], 0, "{done}");
    assert_eq!(listed_ids(&done), only(&carla_id));
    // All or nothing: one unknown id, one guest's, and nothing is added for
    // the good ones beside them.
    let (status, done) = rig.allow("delta", &[nobody]);
    assert_eq!(status, 404, "{done}");
    let (status, done) = rig.allow("delta", &[&other, nobody]);
    assert_eq!(status, 404, "{done}");
    let (status, done) = rig.allow("delta", &[&guest_id]);
    assert_eq!(status, 409, "{done}");
    let (status, done) = rig.allow("delta", &[&other, &guest_id]);
    assert_eq!(status, 409, "{done}");
    let (status, done) = rig.allow("delta", &[&guest_id, &other]);
    assert_eq!(status, 409, "{done}");
    assert_eq!(
        listed_ids(&rig.detail("delta")),
        only(&carla_id),
        "nothing added"
    );
    assert_eq!(rig.row("delta")["listed"], 1);
    // The shapes a request may not have.
    let many: Vec<String> = (0..201)
        .map(|n| format!("0199aaaa-0000-7000-8000-{n:012}"))
        .collect();
    let many: Vec<&str> = many.iter().map(String::as_str).collect();
    for path in ["allow", "disallow"] {
        let at = format!("/admin/llm/access/delta/{path}");
        rig.expect(400, "POST", &at, r#"{"accounts":[]}"#);
        rig.expect(400, "POST", &at, r#"{"accounts":["not-a-uuid"]}"#);
        rig.expect(400, "POST", &at, r#"{"accounts":[1]}"#);
        rig.expect(400, "POST", &at, "{}");
        rig.expect(
            400,
            "POST",
            &at,
            r#"{"accounts":["0199aaaa-0000-7000-8000-000000000001"],"x":1}"#,
        );
        rig.expect(400, "POST", &at, &json!({ "accounts": many }).to_string());
    }
    assert_eq!(listed_ids(&rig.detail("delta")), only(&carla_id));
    // Two together, and the edge of the range: 200 are taken.
    let (status, done) = rig.allow("delta", &[&dora_id, &other]);
    assert_eq!(status, 200, "{done}");
    assert_eq!(done["added"], 2, "{done}");
    assert_eq!(rig.row("delta")["listed"], 3);
    // Disallow takes off exactly those named; the policy stays.
    let (status, done) = rig.disallow("delta", &[&other, nobody, &guest_id]);
    assert_eq!(status, 200, "{done}");
    assert_eq!(done["removed"], 1, "{done}");
    assert_eq!(done["policy"], "listed", "{done}");
    let (_, done) = rig.disallow("delta", &[&other]);
    assert_eq!(done["removed"], 0, "{done}");

    // (h) A player deleting their own account takes it off every list.
    rig.allow("epsilon", &[&dora_id]);
    assert_eq!(listed_ids(&rig.detail("delta")).len(), 2);
    assert_eq!(rig.row("epsilon")["listed"], 1);
    let (status, body) = http(rig.port, "DELETE", "/account", Some(&dora), "{}");
    assert_eq!(status, 204, "{body}");
    assert_eq!(listed_ids(&rig.detail("delta")), only(&carla_id));
    assert_eq!(rig.detail("epsilon")["accounts"], json!([]));
    assert_eq!(rig.row("epsilon")["listed"], 0);
    assert_eq!(
        rig.gw.scalar(&format!(
            "SELECT count(*) FROM llm_profile_allow WHERE account_id = '{dora_id}'"
        )),
        0,
        "the cascade"
    );

    // (j) The type-ahead.
    let found = |q: &str| -> Vec<Value> {
        let v = rig.expect(200, "GET", &format!("/admin/accounts/lookup?q={q}"), "");
        v["accounts"].as_array().unwrap().clone()
    };
    let names = |q: &str| -> Vec<String> {
        let mut names: Vec<String> = found(q)
            .iter()
            .map(|a| a["username"].as_str().unwrap().to_string())
            .collect();
        names.sort();
        names
    };
    assert!(found("").is_empty());
    assert!(found("p").is_empty(), "fewer than two characters");
    assert!(found("%25").is_empty());
    rig.expect(200, "GET", "/admin/accounts/lookup", "");
    // Ten at most, never a guest's, however many there are.
    let ten = found("pfx");
    assert_eq!(ten.len(), 10, "twelve share the prefix: {ten:?}");
    assert!(
        ten.iter()
            .all(|a| a["username"].as_str().unwrap().starts_with("pfx")),
        "{ten:?}"
    );
    assert!(
        ten.iter().all(|a| a["id"] != guest_id.as_str()),
        "a guest is no candidate"
    );
    assert!(
        ten.iter()
            .all(|a| a["display_name"].as_str().unwrap().starts_with("Pfx"))
    );
    assert!(
        ten[0]["handle"]
            .as_str()
            .unwrap()
            .starts_with(ten[0]["display_name"].as_str().unwrap()),
        "{ten:?}"
    );
    assert!(ten[0]["handle"].as_str().unwrap().contains('#'));
    assert_eq!(names("pfx1"), ["pfx10", "pfx11", "pfx12"]);
    assert_eq!(
        names("pfxv"),
        Vec::<String>::new(),
        "the guest 'Pfxvisitor'"
    );
    assert_eq!(
        names("Pfx1"),
        ["pfx10", "pfx11", "pfx12"],
        "case-insensitive"
    );
    assert_eq!(names("PFX05"), ["pfx05"]);
    // By username or by display name, from the start only.
    assert_eq!(names("quu"), ["quux77"]);
    assert_eq!(names("QUUX"), ["quux77"]);
    assert_eq!(names("mir"), ["quux77"], "by display name");
    assert_eq!(names("MIRA"), ["quux77"]);
    assert_eq!(
        names("ux77"),
        Vec::<String>::new(),
        "a prefix, not a substring"
    );
    assert_eq!(names("rabel"), Vec::<String>::new());
    assert_eq!(names("car"), ["carla"]);
    assert_eq!(
        names("dor"),
        Vec::<String>::new(),
        "a deleted account is gone"
    );
    // `%` and `_` are what they say.
    assert_eq!(names("ab_"), ["ab_cd"], "underscore is no wildcard");
    assert_eq!(names("ab"), ["ab_cd", "abxcd"]);
    assert!(found("p_").is_empty());
    assert!(found("pfx%25").is_empty());
    assert!(found("%25x").is_empty());
    assert!(found("%25%25").is_empty());
    assert!(found("pf%25").is_empty(), "no wildcard");
    // The length of a search is bounded.
    assert!(found(&"a".repeat(100)).is_empty());
    rig.expect(
        400,
        "GET",
        &format!("/admin/accounts/lookup?q={}", "a".repeat(101)),
        "",
    );

    // Audit: every change, by action, with the profile and the ids; no name.
    let logs = rig.gw.logs();
    let audit = |action: &str| -> Vec<String> {
        logs.lines()
            .filter(|l| l.contains(action) && l.contains("baylee_gateway::audit"))
            .map(str::to_string)
            .collect()
    };
    let policies = audit("llm.access.policy");
    assert!(policies.len() >= 10, "one per change: {}", policies.len());
    assert!(
        policies
            .iter()
            .any(|l| l.contains("alpha") && l.contains("registered")),
        "{policies:?}"
    );
    assert!(policies.iter().all(|l| l.contains("ada")));
    let allows = audit("llm.access.allow");
    assert!(
        allows
            .iter()
            .any(|l| l.contains("delta") && l.contains(&carla_id)),
        "{allows:?}"
    );
    let disallows = audit("llm.access.disallow");
    assert!(
        disallows
            .iter()
            .any(|l| l.contains("delta") && l.contains(&other)),
        "{disallows:?}"
    );
    for line in policies.iter().chain(&allows).chain(&disallows) {
        let line = line.to_lowercase();
        for name in ["carla", "dora", "pfx01", "quux77", "mirabel"] {
            assert!(!line.contains(name), "{name} in an audit line: {line}");
        }
    }
    assert!(
        allows.iter().chain(&disallows).all(|l| l.contains("ada")),
        "{allows:?} {disallows:?}"
    );
    // A refused change leaves no line: the 409 that named a guest.
    assert!(
        allows.iter().all(|l| !l.contains(&guest_id)),
        "a refused allow was audited: {allows:?}"
    );
}
