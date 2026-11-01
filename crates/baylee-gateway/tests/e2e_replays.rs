//! Replays (`docs/protocol.md` §"Replays"): `GET /replays` lists the hosted
//! games an account sat at, and `POST /replays/{game}/tape` plays one again
//! on this build and answers the tape one seat (or the public) was sent.
//!
//! Everything is real but the people: a gateway process, an agent that runs
//! real engines in this process (a job is one of them), and the record a
//! game played over the real sockets leaves in the gateway's own schema. A
//! test that needs a record the route only looks at the size or the seat of
//! (an oversized one, one of a game that is still going) writes the row by
//! hand, which says so.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use std::collections::BTreeSet;
use std::io::Read as _;

use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_protocol::tape::{self, Line, StopKind};
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{GameStatic, PlayerView, SpectatorView};
use common::{Gateway, Socket, attach_agent, http, http_raw, json_field, login, spawn_gateway};
use futures_util::SinkExt as _;
use prost::Message as _;
use serde_json::{Value, json};

/// The body of an ordinary tape request from a seat.
const SEAT: &str = r#"{"perspective":"seat"}"#;

// ------------------------------------------------------------- the games

fn make_deck(port: u16, token: &str, card: &str) -> String {
    let body = json!({ "name": card, "cards": [format!("60 {card}")] }).to_string();
    let (status, body) = http(port, "POST", "/decks", Some(token), &body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

async fn send_action(ws: &mut Socket, action: &PlayerAction) {
    let envelope = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: String::new(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(action).unwrap(),
        })),
    };
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        envelope.encode_to_vec().into(),
    ))
    .await
    .expect("send an action");
}

/// Reads a seat socket until `pick` has something to say.
async fn read_until<T>(
    ws: &mut Socket,
    what: &str,
    mut pick: impl FnMut(v1::envelope::Msg) -> Option<T>,
) -> T {
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(ws).await {
                Some(msg) => {
                    if let Some(found) = pick(msg) {
                        return found;
                    }
                }
                None => panic!("the socket closed before {what}"),
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("no {what} within the wait budget"))
}

/// The seat's first question, which must be its opening mulligan.
async fn first_question(ws: &mut Socket) -> Pending {
    read_until(ws, "a question", |msg| match msg {
        v1::envelope::Msg::ChoiceRequest(req) => {
            Some(serde_json::from_slice(&req.pending_json).expect("a pending question"))
        }
        _ => None,
    })
    .await
}

/// Waits until the game's record is complete in the store and the engine
/// has said how it ended (the winners are noted after the last piece).
async fn ended(gw: &Gateway, game: &str) {
    for _ in 0..common::WAIT_TRIES {
        let done = gw.scalar(&format!(
            "SELECT count(*) FROM game_record \
             WHERE game_id = '{game}' AND complete AND winners IS NOT NULL"
        ));
        if done == 1 {
            return;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    panic!("the game's record never completed with its end noted");
}

/// Two accounts, `60 Island` against `60 Swamp`, a room, both ready, the
/// host starts: the game's id and each seat's token.
fn start_two_seats(port: u16, alice: &str, bob: &str) -> (String, String, String) {
    let decks = [
        make_deck(port, alice, "Island"),
        make_deck(port, bob, "Swamp"),
    ];
    let create = format!("{{\"deck_id\":\"{}\",\"mode\":\"open\"}}", decks[0]);
    let (status, body) = http(port, "POST", "/lobby/games", Some(alice), &create);
    assert_eq!(status, 200, "create open game: {body}");
    let game = json_field(&body, "game_id").to_string();
    let seat_a = json_field(&body, "seat_token").to_string();
    let join = format!("{{\"deck_id\":\"{}\"}}", decks[1]);
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/join"),
        Some(bob),
        &join,
    );
    assert_eq!(status, 200, "join: {body}");
    let seat_b = json_field(&body, "seat_token").to_string();
    for token in [alice, bob] {
        let (status, body) = http(
            port,
            "POST",
            &format!("/lobby/games/{game}/ready"),
            Some(token),
            "{}",
        );
        assert_eq!(status, 200, "ready: {body}");
    }
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/start"),
        Some(alice),
        "",
    );
    assert_eq!(status, 200, "start: {body}");
    (game, seat_a, seat_b)
}

/// Alice (seat 0, `60 Island`) and Bob (seat 1, `60 Swamp`) play a game:
/// both keep their hands, turn one begins and Alice concedes. Returns the
/// game's id once its record is whole in the store.
///
/// Conceding before turn one would leave the turn the engine reports at 0,
/// which the gateway stores as `NULL`; the list's `turn` is asked about, so
/// the game is played far enough to have one.
async fn two_humans_to_the_end(gw: &Gateway, alice: &str, bob: &str) -> String {
    let port = gw.port;
    let (game, seat_a, seat_b) = start_two_seats(port, alice, bob);
    let mut ws_a = common::dial_seat(port, &game, &seat_a).await;
    let mut ws_b = common::dial_seat(port, &game, &seat_b).await;
    for ws in [&mut ws_a, &mut ws_b] {
        let first = first_question(ws).await;
        assert!(
            matches!(first, Pending::Mulligan { .. }),
            "a seat's first question is its mulligan: {first:?}"
        );
        send_action(ws, &PlayerAction::MulliganKeep).await;
    }
    // Turn one has begun once a seat is shown a view of it.
    read_until(&mut ws_a, "turn one", |msg| match msg {
        v1::envelope::Msg::StateDelta(delta) => {
            let view: PlayerView = serde_json::from_slice(&delta.view_json).ok()?;
            (view.turn >= 1).then_some(())
        }
        _ => None,
    })
    .await;
    send_action(&mut ws_a, &PlayerAction::Concede).await;
    ended(gw, &game).await;
    game
}

/// A one-seat game against the house, started and left open: its id and the
/// seat's token.
fn start_ai_game(port: u16, token: &str, extra: &str) -> (String, String) {
    let deck_id = make_deck(port, token, "Swamp");
    let create = format!("{{\"deck_id\":\"{deck_id}\",\"mode\":\"ai\"{extra}}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(token), &create);
    assert_eq!(status, 200, "create ai game: {body}");
    (
        json_field(&body, "game_id").to_string(),
        json_field(&body, "seat_token").to_string(),
    )
}

/// Sits at an open game's table, waits for the curtain and concedes, and
/// returns once the record is whole.
async fn concede_and_wait(gw: &Gateway, game: &str, seat_token: &str) {
    let mut ws = common::dial_seat(gw.port, game, seat_token).await;
    read_until(&mut ws, "the curtain", |msg| match msg {
        v1::envelope::Msg::Curtain(_) => Some(()),
        _ => None,
    })
    .await;
    send_action(&mut ws, &PlayerAction::Concede).await;
    ended(gw, game).await;
}

/// A game against the house, played to its end by a concession: its id.
async fn a_finished_game(gw: &Gateway, token: &str, extra: &str) -> String {
    let (game, seat_token) = start_ai_game(gw.port, token, extra);
    concede_and_wait(gw, &game, &seat_token).await;
    game
}

// ------------------------------------------------------------ the tapes

/// A tape read back.
struct Tape {
    header: tape::Header,
    statics: Vec<GameStatic>,
    views: Vec<tape::View>,
    overs: usize,
    stop: tape::Stop,
    raw: String,
}

impl Tape {
    /// The seat-perspective views, decoded.
    fn player_views(&self) -> Vec<PlayerView> {
        self.views
            .iter()
            .map(|v| serde_json::from_str(v.json.get()).expect("a PlayerView"))
            .collect()
    }

    /// The public-perspective views, decoded.
    fn spectator_views(&self) -> Vec<SpectatorView> {
        self.views
            .iter()
            .map(|v| serde_json::from_str(v.json.get()).expect("a SpectatorView"))
            .collect()
    }
}

/// An answer's body as a tape: gzip members one after another, JSON Lines,
/// a header first and a `stop` last.
fn read_tape(body: &[u8]) -> Tape {
    let mut raw = String::new();
    flate2::read::MultiGzDecoder::new(body)
        .read_to_string(&mut raw)
        .expect("a tape is gzip");
    let mut lines = Vec::new();
    for text in raw.lines().filter(|l| !l.is_empty()) {
        lines.push(tape::parse_line(text).unwrap_or_else(|e| panic!("{e}: {text}")));
    }
    let mut lines = lines.into_iter();
    let Some(Line::Header(header)) = lines.next() else {
        panic!("a tape opens with its header");
    };
    let (mut statics, mut views, mut overs, mut stop) = (Vec::new(), Vec::new(), 0, None);
    for line in lines {
        assert!(stop.is_none(), "a line after the stop line");
        match line {
            Line::Header(_) => panic!("a second header"),
            Line::Static { json, .. } => {
                statics.push(serde_json::from_str(json.get()).expect("a GameStatic"));
            }
            Line::View(view) => views.push(view),
            // The engine-server's session never sends a curtain to a
            // projection, so none is expected; one is not an error.
            Line::Curtain { .. } => {}
            Line::Over { .. } => overs += 1,
            Line::Stop(s) => stop = Some(s),
        }
    }
    Tape {
        header,
        statics,
        views,
        overs,
        stop: stop.expect("a tape closes with its stop line"),
        raw,
    }
}

fn ask_tape(port: u16, token: &str, game: &str, body: &str) -> (u16, String, Vec<u8>) {
    http_raw(
        port,
        "POST",
        &format!("/replays/{game}/tape"),
        Some(token),
        body,
    )
}

/// A tape that was answered, read.
fn tape_of(port: u16, token: &str, game: &str, body: &str) -> Tape {
    let (status, head, bytes) = ask_tape(port, token, game, body);
    assert_eq!(status, 200, "a tape: {}", text(&bytes));
    assert!(head.contains("content-type: application/gzip"), "{head}");
    read_tape(&bytes)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn list(port: u16, token: &str, query: &str) -> Value {
    let (status, body) = http(port, "GET", &format!("/replays{query}"), Some(token), "");
    assert_eq!(status, 200, "the list: {body}");
    serde_json::from_str(&body).expect("json")
}

fn health(port: u16) -> Value {
    let (_, body) = http(port, "GET", "/health", None, "");
    serde_json::from_str(&body).expect("json")
}

/// The raw text of each view a tape holds.
fn view_texts(tape: &Tape) -> Vec<String> {
    tape.views.iter().map(|v| v.json.get().to_owned()).collect()
}

/// Every object id a JSON value names, under any `id` key.
fn ids_under(value: &Value, out: &mut BTreeSet<u64>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                if key == "id" {
                    out.extend(inner.as_u64());
                }
                ids_under(inner, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|inner| ids_under(inner, out)),
        _ => {}
    }
}

/// Every object id anywhere in a tape's views, log tails and payloads.
fn ids_anywhere(tape: &Tape) -> BTreeSet<u64> {
    let mut ids = BTreeSet::new();
    for view in &tape.views {
        ids_under(&serde_json::from_str(view.json.get()).unwrap(), &mut ids);
        if let Some(log) = &view.log {
            ids_under(&serde_json::from_str(log.get()).unwrap(), &mut ids);
        }
    }
    ids
}

/// The ids of the cards the seat's own `hand` held, over the whole tape.
fn hand_ids_of(tape: &Tape) -> BTreeSet<u64> {
    let mut ids = BTreeSet::new();
    for view in &tape.views {
        let value: Value = serde_json::from_str(view.json.get()).unwrap();
        ids_under(&value["hand"], &mut ids);
    }
    ids
}

/// The ids of the cards the hand of `player` was shown with, beside the
/// seat's own, over the whole tape (`public: false` entries only).
fn shown_hand_ids_of(tape: &Tape, player: u64) -> BTreeSet<u64> {
    let mut ids = BTreeSet::new();
    for view in &tape.views {
        let value: Value = serde_json::from_str(view.json.get()).unwrap();
        for hand in value["shared_hands"].as_array().into_iter().flatten() {
            if hand["player"] == player && hand["public"] == false {
                ids_under(&hand["cards"], &mut ids);
            }
        }
    }
    ids
}

/// The players whose hands a tape shows beside the seat's, in any view.
fn players_shown(tape: &Tape) -> BTreeSet<u64> {
    let mut players = BTreeSet::new();
    for view in &tape.views {
        let value: Value = serde_json::from_str(view.json.get()).unwrap();
        for hand in value["shared_hands"].as_array().into_iter().flatten() {
            if hand["public"] == false {
                players.extend(hand["player"].as_u64());
            }
        }
    }
    players
}

/// `PUT /replays/{game}/share`.
fn put_share(port: u16, token: &str, game: &str, body: &str) -> (u16, String) {
    http(
        port,
        "PUT",
        &format!("/replays/{game}/share"),
        Some(token),
        body,
    )
}

/// The account's seat says whether it shares: it must be answered.
fn set_shared(port: u16, token: &str, game: &str, shared: bool) {
    let (status, body) = put_share(port, token, game, &json!({ "shared": shared }).to_string());
    assert_eq!(status, 200, "share {shared}: {body}");
    let said: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(said["shared"], shared, "{body}");
}

/// `PUT /replays/{game}/archived`.
fn put_archived(port: u16, token: &str, game: &str, body: &str) -> (u16, String) {
    http(
        port,
        "PUT",
        &format!("/replays/{game}/archived"),
        Some(token),
        body,
    )
}

/// The account archives (or takes back) a game: it must be answered.
fn archive(port: u16, token: &str, game: &str, archived: bool) {
    let (status, body) = put_archived(
        port,
        token,
        game,
        &json!({ "archived": archived }).to_string(),
    );
    assert_eq!(status, 200, "archive {archived}: {body}");
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap(),
        json!({ "archived": archived })
    );
}

/// The ids of the games a list page names, in order.
fn game_ids(page: &Value) -> Vec<String> {
    page["games"]
        .as_array()
        .expect("games")
        .iter()
        .map(|g| g["game"].as_str().expect("id").to_owned())
        .collect()
}

// --------------------------------------------------------- hand-written rows

/// An account's id, which no route says.
fn account_id(gw: &Gateway, username: &str) -> String {
    gw.text(&format!(
        "SELECT id::text FROM account WHERE username = '{username}'"
    ))
}

/// A record row written by hand, with `account` at its seat 0: for the
/// answers that only look at its size, its end or its seats. `build`,
/// where given, is the build the row says played it.
fn seed_record(gw: &Gateway, game: &str, account: &str, bytes: i64, build: Option<&str>) {
    let build = build.map_or("NULL".to_owned(), |b| format!("'{b}'"));
    gw.sql(&format!(
        "INSERT INTO game_record (game_id, bytes, complete, build, checked) \
         VALUES ('{game}', {bytes}, true, {build}, true)"
    ));
    gw.sql(&format!(
        "INSERT INTO game_record_seat (game_id, seat, account_id, seated) \
         VALUES ('{game}', 0, '{account}', 'person')"
    ));
}

const MIB: i64 = 1024 * 1024;

// ----------------------------------------------------------------- tests

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn a_finished_game_is_listed_for_each_player_as_they_sat_there() {
    let gw = spawn_gateway("replays-list");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let alice = login(port, "alice", "AliceReplay");
    let bob = login(port, "bob", "BobReplay");
    let carol = login(port, "carol", "CarolReplay");
    let game = two_humans_to_the_end(&gw, &alice, &bob).await;

    // Alice conceded: she lost, Bob won; each reads the table from its seat.
    let mine = list(port, &alice, "");
    assert_eq!(mine["total"], 1, "{mine}");
    let row = &mine["games"][0];
    assert_eq!(row["game"], game.as_str());
    assert_eq!(row["seat"], 0);
    assert_eq!(row["result"], "lost", "{row}");
    assert_eq!(row["complete"], true);
    assert_eq!(row["public"], true);
    assert_eq!(row["archived"], false);
    assert_eq!(row["check"], "exact", "the harness engine is this build");
    assert_eq!(row["build"], baylee_build::short());
    assert!(row["turn"].as_u64().is_some_and(|t| t >= 1), "{row}");
    assert!(row["started_at"].as_str().is_some_and(|t| t.ends_with('Z')));
    assert!(row["ended_at"].as_str().is_some_and(|t| t.ends_with('Z')));
    let seats = row["seats"].as_array().expect("seats");
    assert_eq!(seats.len(), 2);
    assert_eq!(seats[0]["seat"], 0);
    assert_eq!(seats[0]["who"], "you");
    assert!(
        seats[0]["name"]
            .as_str()
            .is_some_and(|n| n.starts_with("AliceReplay#")),
        "{row}"
    );
    assert_eq!(seats[0]["colours"], "U", "an Island deck is blue: {row}");
    assert_eq!(seats[1]["seat"], 1);
    assert_eq!(seats[1]["who"], "player");
    assert!(
        seats[1]["name"]
            .as_str()
            .is_some_and(|n| n.starts_with("BobReplay#")),
        "{row}"
    );
    assert!(
        seats[1].get("colours").is_none(),
        "the other deck's colours are not shown: {row}"
    );
    assert!(seats[0]["commander"].is_null() && seats[1]["commander"].is_null());

    let theirs = list(port, &bob, "");
    let row = &theirs["games"][0];
    assert_eq!(theirs["total"], 1);
    assert_eq!(row["game"], game.as_str());
    assert_eq!(row["seat"], 1);
    assert_eq!(row["result"], "won", "{row}");
    let seats = row["seats"].as_array().expect("seats");
    assert_eq!(seats[0]["who"], "player");
    assert!(seats[0].get("colours").is_none(), "{row}");
    assert_eq!(seats[1]["who"], "you");
    assert_eq!(seats[1]["colours"], "B", "a Swamp deck is black: {row}");

    // Somebody who did not sit there has no games, said exactly.
    let (status, body) = http(port, "GET", "/replays", Some(&carol), "");
    assert_eq!(status, 200);
    assert_eq!(body, r#"{"games":[],"total":0}"#);

    // And nobody signed out has a list.
    assert_eq!(http(port, "GET", "/replays", None, "").0, 401);
    assert_eq!(
        http(port, "GET", "/replays", Some("not-a-session"), "").0,
        401
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_list_pages_newest_first_and_caps_a_page_at_fifty() {
    let gw = spawn_gateway("replays-paging");
    let port = gw.port;
    let pager = login(port, "pager", "PagerReplay");
    let id = account_id(&gw, "pager");
    // Fifty-five games by hand, g001 the newest: the cap is only seen past
    // fifty, and no test plays that many.
    gw.sql(
        "INSERT INTO game_record (game_id, started_at, ended_at, complete, winners, turn) \
         SELECT 'g' || lpad(i::text, 3, '0'), \
                now() - make_interval(mins => i), now() - make_interval(mins => i), \
                true, ARRAY[0]::smallint[], 3 \
         FROM generate_series(1, 55) AS i",
    );
    gw.sql(&format!(
        "INSERT INTO game_record_seat (game_id, seat, account_id, seated) \
         SELECT 'g' || lpad(i::text, 3, '0'), 0, '{id}', 'person' \
         FROM generate_series(1, 55) AS i"
    ));
    let ids = |page: &Value| -> Vec<String> {
        page["games"]
            .as_array()
            .expect("games")
            .iter()
            .map(|g| g["game"].as_str().expect("id").to_owned())
            .collect()
    };

    let first = list(port, &pager, "");
    assert_eq!(first["total"], 55);
    assert_eq!(ids(&first).len(), 20, "20 is the default page");
    assert_eq!(ids(&first)[0], "g001", "newest first");
    assert_eq!(ids(&first)[19], "g020");

    let page = list(port, &pager, "?offset=2&limit=3");
    assert_eq!(ids(&page), ["g003", "g004", "g005"]);
    assert_eq!(page["total"], 55, "the total is of all, not of the page");

    let capped = list(port, &pager, "?limit=500");
    assert_eq!(ids(&capped).len(), 50, "a page is at most 50");
    assert_eq!(capped["total"], 55);
    let rest = list(port, &pager, "?offset=50&limit=50");
    assert_eq!(ids(&rest), ["g051", "g052", "g053", "g054", "g055"]);
    let past = list(port, &pager, "?offset=55");
    assert_eq!(ids(&past).len(), 0);
    assert_eq!(past["total"], 55);

    // A page of none is a page of one, not of everything.
    assert_eq!(ids(&list(port, &pager, "?limit=0")), ["g001"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one table of cases, read top to bottom
async fn the_list_reads_result_check_and_who_from_the_stored_columns() {
    let gw = spawn_gateway("replays-columns-read");
    let port = gw.port;
    let me = login(port, "reader", "MeRead");
    let _other = login(port, "another", "OtherRead");
    let (me_id, other_id) = (account_id(&gw, "reader"), account_id(&gw, "another"));
    let build = baylee_build::short();
    // (game, complete, winners, build, checked): newest first, in this order.
    let games = [
        ("g1-won", "true", "'{0}'", format!("'{build}'"), "true"),
        ("g2-lost", "true", "'{1}'", format!("'{build}'"), "true"),
        ("g3-draw", "true", "'{}'", format!("'{build}'"), "true"),
        ("g4-unknown", "true", "NULL", format!("'{build}'"), "true"),
        (
            "g5-unfinished",
            "false",
            "NULL",
            format!("'{build}'"),
            "true",
        ),
        (
            "g6-checked",
            "true",
            "'{0}'",
            "'0.0.1+build.1 (0000000000)'".to_owned(),
            "true",
        ),
        (
            "g7-unverified",
            "true",
            "'{0}'",
            "'0.0.1+build.1 (0000000000)'".to_owned(),
            "false",
        ),
        ("g8-nobuild", "true", "'{0}'", "NULL".to_owned(), "false"),
    ];
    for (i, (game, complete, winners, built, checked)) in games.iter().enumerate() {
        gw.sql(&format!(
            "INSERT INTO game_record (game_id, started_at, complete, winners, build, checked) \
             VALUES ('{game}', now() - make_interval(mins => {i}), {complete}, \
                     {winners}::smallint[], {built}, {checked})"
        ));
        gw.sql(&format!(
            "INSERT INTO game_record_seat (game_id, seat, account_id, seated) \
             VALUES ('{game}', 0, '{me_id}', 'person')"
        ));
    }
    // One game with a chair of every kind; the seats of an older record say
    // nothing in `seated`, and the list reads what the links still show.
    for (seat, account, delegate, seated) in [
        (1, "NULL".to_owned(), format!("'{me_id}'"), "'model'"),
        (2, "NULL".to_owned(), "NULL".to_owned(), "'house'"),
        (3, format!("'{other_id}'"), "NULL".to_owned(), "NULL"),
        (4, "NULL".to_owned(), format!("'{me_id}'"), "NULL"),
        (5, "NULL".to_owned(), "NULL".to_owned(), "NULL"),
        (6, "NULL".to_owned(), "NULL".to_owned(), "'person'"),
        (7, format!("'{other_id}'"), "NULL".to_owned(), "'person'"),
    ] {
        gw.sql(&format!(
            "INSERT INTO game_record_seat (game_id, seat, account_id, delegated_by, seated) \
             VALUES ('g1-won', {seat}, {account}, {delegate}, {seated})"
        ));
    }

    let page = list(port, &me, "?limit=50");
    assert_eq!(page["total"], 8);
    let rows = page["games"].as_array().expect("games");
    let by = |game: &str| {
        rows.iter()
            .find(|g| g["game"] == game)
            .unwrap_or_else(|| panic!("{game} is listed"))
    };
    assert_eq!(
        rows.iter()
            .map(|g| g["game"].as_str().unwrap())
            .collect::<Vec<_>>(),
        games.iter().map(|g| g.0).collect::<Vec<_>>(),
        "newest first"
    );
    for (game, result, check) in [
        ("g1-won", "won", "exact"),
        ("g2-lost", "lost", "exact"),
        ("g3-draw", "draw", "exact"),
        ("g4-unknown", "unknown", "exact"),
        ("g5-unfinished", "unfinished", "exact"),
        ("g6-checked", "won", "checked"),
        ("g7-unverified", "won", "unverified"),
        ("g8-nobuild", "won", "unverified"),
    ] {
        let row = by(game);
        assert_eq!(row["result"], result, "{game}: {row}");
        assert_eq!(row["check"], check, "{game}: {row}");
        assert_eq!(row["seat"], 0);
        assert_eq!(row["public"], true);
    }
    assert_eq!(by("g5-unfinished")["complete"], false);
    assert!(by("g5-unfinished")["ended_at"].is_null());
    assert!(by("g8-nobuild")["build"].is_null());
    assert!(by("g1-won")["turn"].is_null());

    let seats = by("g1-won")["seats"].as_array().expect("seats");
    let who: Vec<_> = seats
        .iter()
        .map(|s| (s["who"].as_str().unwrap(), s["name"].as_str().unwrap()))
        .collect();
    assert_eq!(who[0].0, "you");
    assert_eq!(
        who[1..],
        [
            ("model", "Language model"),
            ("house", "House AI"),
            // From before `seated`: an account still linked is a player,
            ("player", who[3].1),
            // a bridge's host link a model, nothing at all the house.
            ("model", "Language model"),
            ("house", "House AI"),
            // A person whose link was cut is a former player.
            ("former", "Former player"),
            ("player", who[7].1),
        ]
    );
    assert!(who[3].1.starts_with("OtherRead#") && who[7].1.starts_with("OtherRead#"));
    // Only the asking account's own seat carries colours, if it has any.
    assert!(
        seats.iter().all(|s| s.get("colours").is_none()),
        "{seats:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn a_seat_is_sent_its_own_view_and_the_public_the_spectators() {
    let gw = spawn_gateway("replays-tapes");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let alice = login(port, "alice", "AliceTape");
    let bob = login(port, "bob", "BobTape");
    let carol = login(port, "carol", "CarolTape");
    let game = two_humans_to_the_end(&gw, &alice, &bob).await;

    // Alice's tape: seat 0, on this build, to the game's end.
    let a = tape_of(port, &alice, &game, SEAT);
    assert_eq!(a.header.tape, tape::TAPE_VERSION);
    assert_eq!(a.header.view, baylee_view::VIEW_VERSION);
    assert_eq!(a.header.build, baylee_build::short());
    assert_eq!(a.header.recorded_by, baylee_build::short());
    assert_eq!(a.header.source, tape::Source::Hosted);
    assert_eq!(a.header.perspective, tape::Perspective::Seat);
    assert_eq!(a.header.seat, 0);
    assert_eq!(a.header.game, game);
    assert_eq!(a.statics.len(), 1, "the payload once");
    assert_eq!(a.statics[0].your_seat, PlayerId::new(0));
    assert!(a.statics[0].seats[0].display_name.starts_with("AliceTape#"));
    assert!(a.statics[0].seats[1].display_name.starts_with("BobTape#"));
    assert!(
        a.statics[0].prints.iter().any(Option::is_none),
        "the opening payload has a hole where the other deck's printing is"
    );
    assert!(!a.views.is_empty());
    assert_eq!(a.overs, 1, "and the game ended once");
    assert_eq!(a.stop.stop, StopKind::End);
    assert!(a.stop.inputs > 0);
    assert!(
        a.stop.exact_to.is_some(),
        "same build: the engine's hash held"
    );
    assert!(a.stop.diverged.is_none());
    assert!(
        a.views.iter().all(|v| v.check == tape::Check::Exact),
        "every frame is the recorded game's"
    );

    // Bob's: seat 1.
    let b = tape_of(port, &bob, &game, SEAT);
    assert_eq!(b.header.seat, 1);
    assert_eq!(b.header.perspective, tape::Perspective::Seat);
    assert_eq!(b.statics[0].your_seat, PlayerId::new(1));
    assert_eq!(b.stop.stop, StopKind::End);

    // Each is sent its own hand and never the other's. Alice's deck is all
    // Island and Bob's all Swamp, so a card in the wrong hand names itself;
    // and the objects of the two hands are different objects.
    let (a_views, b_views) = (a.player_views(), b.player_views());
    assert!(a_views.iter().all(|v| v.seat == PlayerId::new(0)));
    assert!(b_views.iter().all(|v| v.seat == PlayerId::new(1)));
    let hand_ids = |views: &[PlayerView]| -> std::collections::BTreeSet<_> {
        views
            .iter()
            .flat_map(|v| v.hand.iter().map(|card| card.id))
            .collect()
    };
    let (a_hand, b_hand) = (hand_ids(&a_views), hand_ids(&b_views));
    assert!(a_hand.len() >= 7 && b_hand.len() >= 7, "a hand was shown");
    assert!(
        a_hand.is_disjoint(&b_hand),
        "seat 0 was shown a card of seat 1's hand"
    );
    assert!(
        a_views
            .iter()
            .flat_map(|v| &v.hand)
            .all(|card| card.name == "Island"),
        "seat 0's hand is its own"
    );
    assert!(
        b_views
            .iter()
            .flat_map(|v| &v.hand)
            .all(|card| card.name == "Swamp"),
        "seat 1's hand is its own"
    );
    // The other hand is a count.
    assert!(
        a_views
            .iter()
            .any(|v| v.seats.get(1).is_some_and(|s| s.hand_count >= 7)),
        "seat 1's hand reaches seat 0 as a number"
    );

    // Somebody who did not sit there is refused; so is a game that is not.
    let (status, _, body) = ask_tape(port, &carol, &game, SEAT);
    assert_eq!(status, 403, "{}", text(&body));
    let (status, _, body) = ask_tape(port, &alice, "no-such-game", SEAT);
    assert_eq!(status, 404, "{}", text(&body));
    assert_eq!(ask_tape(port, "not-a-session", &game, SEAT).0, 401);

    // The public tape is the spectators' stream, for any seated account,
    // and it is drawn from seat 0 whoever asked.
    for token in [&alice, &bob] {
        let p = tape_of(port, token, &game, r#"{"perspective":"public"}"#);
        assert_eq!(p.header.perspective, tape::Perspective::Public);
        assert_eq!(p.header.seat, 0);
        assert!(!p.spectator_views().is_empty());
        assert_eq!(p.stop.stop, StopKind::End);
        assert!(
            !p.raw.contains("\"hand\":["),
            "no private hand in the public tape"
        );
    }

    // Verify defaults to stop; note is accepted.
    let noted = tape_of(
        port,
        &alice,
        &game,
        r#"{"perspective":"seat","verify":"note"}"#,
    );
    assert_eq!(noted.stop.stop, StopKind::End);

    // A body that is not a tape request is a 400, before any other question.
    for body in [
        "",
        "{}",
        r#"{"perspective":"everything"}"#,
        r#"{"perspective":"seat","verify":"maybe"}"#,
        r#"{"perspective":"seat","hands":"all"}"#,
        // The gateway decides what a tape shows; the request cannot ask.
        r#"{"perspective":"all_hands","hands":[1]}"#,
        r#"{"perspective":"seat","own":true}"#,
        r#"{"perspective":"seat","shared":true}"#,
        "[1]",
    ] {
        let (status, _, answer) = ask_tape(port, &alice, &game, body);
        assert_eq!(status, 400, "{body:?}: {}", text(&answer));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_request_picks_the_seat_drawn_and_the_table_picks_the_account() {
    let gw = spawn_gateway("replays-seat");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let alice = login(port, "alice", "AliceSeat");
    let bob = login(port, "bob", "BobSeat");
    let carol = login(port, "carol", "CarolSeat");
    let game = two_humans_to_the_end(&gw, &alice, &bob).await;

    // Without a seat, Bob's tape is seat 1's, his own.
    let own = tape_of(port, &bob, &game, SEAT);
    assert_eq!(own.header.seat, 1);
    assert!(own.header.own && own.header.hands.is_empty());
    assert!(
        own.player_views()
            .iter()
            .all(|v| v.seat == PlayerId::new(1))
    );
    // Naming his own seat is the same tape.
    let named = tape_of(port, &bob, &game, r#"{"perspective":"seat","seat":1}"#);
    assert_eq!(named.header, own.header);
    assert_eq!(view_texts(&named), view_texts(&own));

    // Naming seat 0 draws the table from Alice's seat: nobody has unticked,
    // so it is her view, exactly as her own tape has it.
    let hers = tape_of(port, &bob, &game, r#"{"perspective":"seat","seat":0}"#);
    assert_eq!(hers.header.seat, 0);
    assert!(hers.header.own, "Alice shares by default");
    assert!(
        hers.player_views()
            .iter()
            .all(|v| v.seat == PlayerId::new(0))
    );
    assert_eq!(hers.statics[0].your_seat, PlayerId::new(0));
    assert!(!hand_ids_of(&hers).is_empty());

    // The seat does not make the account: a stranger is refused whichever
    // seat she names, and a seat the table lacks is a 400, not a table.
    for seat in [0, 1] {
        let body = format!(r#"{{"perspective":"seat","seat":{seat}}}"#);
        assert_eq!(ask_tape(port, &carol, &game, &body).0, 403, "seat {seat}");
    }
    for body in [
        r#"{"perspective":"seat","seat":2}"#,
        r#"{"perspective":"seat","seat":255}"#,
        r#"{"perspective":"seat","seat":256}"#,
        r#"{"perspective":"seat","seat":-1}"#,
        r#"{"perspective":"seat","seat":"1"}"#,
        r#"{"perspective":"all_hands","seat":9}"#,
    ] {
        let (status, _, answer) = ask_tape(port, &bob, &game, body);
        assert_eq!(status, 400, "{body}: {}", text(&answer));
    }
    // The public table has no seat to be drawn from: the seat is ignored.
    let public = tape_of(port, &bob, &game, r#"{"perspective":"public","seat":1}"#);
    assert_eq!(public.header.perspective, tape::Perspective::Public);
    assert_eq!(public.header.seat, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_account_asks_six_tapes_a_minute_and_the_seventh_is_refused() {
    let gw = spawn_gateway("replays-rate");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let rhea = login(port, "rhea", "RheaRate");
    let other = login(port, "otto", "OttoRate");
    let game = a_finished_game(&gw, &rhea, "").await;
    // Played for real, so each ask costs an engine's run: six of them, one
    // after another.
    for i in 0..6 {
        let (status, _, body) = ask_tape(port, &rhea, &game, SEAT);
        assert_eq!(status, 200, "ask {i}: {}", text(&body));
    }
    let (status, _, body) = ask_tape(port, &rhea, &game, SEAT);
    assert_eq!(status, 429, "the seventh: {}", text(&body));
    // A refusal before an account sat there costs nothing: another account's
    // 403 is not an ask, and its own budget is its own.
    let (status, _, _) = ask_tape(port, &other, &game, SEAT);
    assert_eq!(status, 403);
    let second = a_finished_game(&gw, &other, "").await;
    let (status, _, body) = ask_tape(port, &other, &second, SEAT);
    assert_eq!(
        status,
        200,
        "another account is not held to this one's count: {}",
        text(&body)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_account_has_two_jobs_at_once_and_a_third_is_refused() {
    let gw = common::spawn_gateway_with(
        "replays-twice",
        &[("BAYLEE_REPLAY_JOB_SECS", "6".to_owned())],
    );
    let port = gw.port;
    let (_agent, probe) = common::attach_agent_probed(&gw).await;
    let rhea = login(port, "rhea", "RheaTwice");
    let game = a_finished_game(&gw, &rhea, "").await;

    // Two engines that never answer.
    probe.set_mute(true);
    let asks: Vec<_> = (0..2)
        .map(|_| {
            let (token, game) = (rhea.clone(), game.clone());
            std::thread::spawn(move || ask_tape(port, &token, &game, SEAT).0)
        })
        .collect();
    // Both are running when the agent has two engines.
    let mut running = 0;
    for _ in 0..common::WAIT_TRIES {
        running = health(port)["agents"]["games"].as_i64().unwrap_or(0);
        if running == 2 {
            break;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    assert_eq!(running, 2, "two jobs run");
    let (status, _, body) = ask_tape(port, &rhea, &game, SEAT);
    assert_eq!(status, 429, "a third at once: {}", text(&body));

    // They time out (504); then the slots are free again.
    for ask in asks {
        assert_eq!(ask.join().expect("the ask's thread"), 504);
    }
    probe.set_mute(false);
    let (status, _, body) = ask_tape(port, &rhea, &game, SEAT);
    assert_eq!(status, 200, "a slot given back: {}", text(&body));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_engine_that_never_answers_is_a_504_and_the_next_ask_works() {
    let gw = common::spawn_gateway_with(
        "replays-timeout",
        &[("BAYLEE_REPLAY_JOB_SECS", "2".to_owned())],
    );
    let port = gw.port;
    let (_agent, probe) = common::attach_agent_probed(&gw).await;
    let rhea = login(port, "rhea", "RheaTimeout");
    let game = a_finished_game(&gw, &rhea, "").await;

    probe.set_mute(true);
    let began = std::time::Instant::now();
    let (status, _, body) = ask_tape(port, &rhea, &game, SEAT);
    assert_eq!(status, 504, "{}", text(&body));
    assert!(
        began.elapsed() >= std::time::Duration::from_secs(2),
        "not before BAYLEE_REPLAY_JOB_SECS: {:?}",
        began.elapsed()
    );
    // The job is gone from the agent's engines: it was told to stop.
    assert_eq!(health(port)["agents"]["games"], 0);

    probe.set_mute(false);
    let (status, _, body) = ask_tape(port, &rhea, &game, SEAT);
    assert_eq!(status, 200, "the slot was freed: {}", text(&body));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn replays_are_off_with_no_jobs_and_say_so_in_info() {
    let off = common::spawn_gateway_with("replays-off", &[("BAYLEE_REPLAY_JOBS", "0".to_owned())]);
    let (status, info) = http(off.port, "GET", "/info", None, "");
    assert_eq!(status, 200, "{info}");
    let info: Value = serde_json::from_str(&info).expect("json");
    assert!(info["replays"].is_null(), "{info}");
    let token = login(off.port, "rhea", "RheaOff");
    // A good request, and 503 before a game is looked for.
    let (status, _, body) = ask_tape(off.port, &token, "any-game", SEAT);
    assert_eq!(status, 503, "{}", text(&body));
    assert!(text(&body).contains("off"), "{}", text(&body));
    // The body is still read first.
    assert_eq!(ask_tape(off.port, &token, "any-game", "{}").0, 400);

    let on = spawn_gateway("replays-on");
    let (_, info) = http(on.port, "GET", "/info", None, "");
    let info: Value = serde_json::from_str(&info).expect("json");
    assert_eq!(info["replays"], json!({ "version": 1 }), "{info}");
    assert_eq!(info["replays"]["version"], tape::TAPE_VERSION);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn while_admission_is_held_no_tape_is_made() {
    let hold = std::env::temp_dir().join(format!("baylee-replays-hold-{}", std::process::id()));
    let _ = std::fs::remove_file(&hold);
    let gw = common::spawn_gateway_with(
        "replays-hold",
        &[("BAYLEE_ADMISSION_HOLD", hold.display().to_string())],
    );
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let rhea = login(port, "rhea", "RheaHold");
    let game = a_finished_game(&gw, &rhea, "").await;

    std::fs::write(&hold, b"").unwrap();
    let (status, _, body) = ask_tape(port, &rhea, &game, SEAT);
    assert_eq!(status, 503, "{}", text(&body));
    assert!(text(&body).contains("being updated"), "{}", text(&body));
    // The list is a read of the store, not a job.
    assert_eq!(list(port, &rhea, "")["total"], 1);

    std::fs::remove_file(&hold).unwrap();
    let (status, _, body) = ask_tape(port, &rhea, &game, SEAT);
    assert_eq!(status, 200, "lifted: {}", text(&body));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn with_no_agent_there_is_no_engine_to_make_a_tape() {
    // No agent ever attaches, so the record is written by hand.
    let gw = spawn_gateway("replays-no-agent");
    let port = gw.port;
    let rhea = login(port, "rhea", "RheaNoAgent");
    let id = account_id(&gw, "rhea");
    seed_record(&gw, "g-small", &id, 100, Some(baylee_build::short()));
    let (status, _, body) = ask_tape(port, &rhea, "g-small", SEAT);
    assert_eq!(status, 503, "{}", text(&body));
    assert!(text(&body).contains("no engine"), "{}", text(&body));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_record_past_eight_mebibytes_is_not_replayed() {
    // No agent: a record at the bound goes on to ask for an engine (503), a
    // byte past it is refused before anything is loaded (413).
    let gw = spawn_gateway("replays-big");
    let port = gw.port;
    let rhea = login(port, "rhea", "RheaBig");
    let id = account_id(&gw, "rhea");
    seed_record(&gw, "g-at", &id, 8 * MIB, None);
    seed_record(&gw, "g-over", &id, 8 * MIB + 1, None);
    let (status, _, body) = ask_tape(port, &rhea, "g-at", SEAT);
    assert_eq!(status, 503, "at the bound: {}", text(&body));
    assert!(text(&body).contains("no engine"), "{}", text(&body));
    let (status, _, body) = ask_tape(port, &rhea, "g-over", SEAT);
    assert_eq!(status, 413, "{}", text(&body));
    // Its row is listed all the same.
    assert_eq!(list(port, &rhea, "")["total"], 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_game_still_being_played_is_not_replayed() {
    let gw = spawn_gateway("replays-live");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let rhea = login(port, "rhea", "RheaLive");
    let id = account_id(&gw, "rhea");
    let (game, seat_token) = start_ai_game(port, &rhea, "");

    // The game's record is only opened once a piece of it is due, so the row
    // is written by hand for the game that is going on; its engine, when it
    // does write, finds the row there.
    gw.sql(&format!(
        "INSERT INTO game_record (game_id, bytes, complete, build, checked) \
         VALUES ('{game}', 0, false, '{}', true)",
        baylee_build::short()
    ));
    gw.sql(&format!(
        "INSERT INTO game_record_seat (game_id, seat, account_id, seated) \
         VALUES ('{game}', 0, '{id}', 'person')"
    ));
    let (status, _, body) = ask_tape(port, &rhea, &game, SEAT);
    assert_eq!(status, 409, "{}", text(&body));

    // Ended, it is asked for as any other: not a 409 any more.
    concede_and_wait(&gw, &game, &seat_token).await;
    let (status, _, body) = ask_tape(port, &rhea, &game, SEAT);
    assert_eq!(status, 200, "{}", text(&body));
    assert_eq!(read_tape(&body).stop.stop, StopKind::End);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_player_who_left_is_a_former_player_and_the_game_still_replays() {
    let gw = spawn_gateway("replays-departed");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let alice = login(port, "alice", "AliceGone");
    let bob = login(port, "bob", "BobStays");
    let game = two_humans_to_the_end(&gw, &alice, &bob).await;

    let (status, body) = http(port, "DELETE", "/account", Some(&alice), "{}");
    assert_eq!(status, 204, "{body}");
    // The deleted account's session is no session.
    assert_eq!(http(port, "GET", "/replays", Some(&alice), "").0, 401);
    assert_eq!(ask_tape(port, &alice, &game, SEAT).0, 401);

    // Bob still has the game, and Alice's seat is a former player's: not
    // the house, which her chair was recorded as having been a person's.
    let page = list(port, &bob, "");
    assert_eq!(page["total"], 1, "{page}");
    let row = &page["games"][0];
    assert_eq!(row["game"], game.as_str());
    assert_eq!(row["seat"], 1);
    assert_eq!(row["result"], "won");
    let seats = row["seats"].as_array().expect("seats");
    assert_eq!(seats[0]["who"], "former", "{row}");
    assert_eq!(seats[0]["name"], "Former player", "{row}");
    assert!(seats[0].get("colours").is_none());
    assert_eq!(seats[1]["who"], "you");
    // Card data stays with the record when the account link is cut.
    assert_eq!(
        gw.text(&format!(
            "SELECT colours FROM game_record_seat WHERE game_id = '{game}' AND seat = 0"
        )),
        "U"
    );

    // His tape works, and names her as the list does.
    let tape = tape_of(port, &bob, &game, SEAT);
    assert_eq!(tape.header.seat, 1);
    assert_eq!(tape.stop.stop, StopKind::End);
    assert_eq!(tape.statics[0].seats[0].display_name, "Former player");
    assert!(
        tape.statics[0].seats[1]
            .display_name
            .starts_with("BobStays#")
    );
    let public = tape_of(port, &bob, &game, r#"{"perspective":"public"}"#);
    assert_eq!(public.statics[0].seats[0].display_name, "Former player");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_job_counts_against_the_room_an_agent_has() {
    let gw = spawn_gateway("replays-room");
    let port = gw.port;
    // Room for one engine: a game, or a job.
    let (_agent, _probe) = common::attach_agent_with_capacity(&gw, 1).await;
    let rhea = login(port, "rhea", "RheaRoom");
    let first = a_finished_game(&gw, &rhea, "").await;

    // A second game takes the agent's one place and is not over.
    let (second, seat_token) = start_ai_game(port, &rhea, "");
    let (status, _, body) = ask_tape(port, &rhea, &first, SEAT);
    assert_eq!(status, 503, "{}", text(&body));
    assert!(text(&body).contains("no engine"), "{}", text(&body));

    // It ends, the place is free, and the job runs.
    concede_and_wait(&gw, &second, &seat_token).await;
    let mut answer = ask_tape(port, &rhea, &first, SEAT);
    for _ in 0..common::WAIT_TRIES {
        if answer.0 != 503 {
            break;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
        answer = ask_tape(port, &rhea, &first, SEAT);
    }
    assert_eq!(answer.0, 200, "{}", text(&answer.2));
    assert_eq!(read_tape(&answer.2).stop.stop, StopKind::End);
    // A job is not a game: nothing of it is in the lobby or the records.
    assert_eq!(gw.scalar("SELECT count(*) FROM game_record"), 2);
    assert_eq!(health(port)["games"]["running"], 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_game_leaves_what_the_list_reads_in_the_store() {
    let gw = spawn_gateway("replays-columns");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let alice = login(port, "alice", "AliceCols");
    let bob = login(port, "bob", "BobCols");
    let game = two_humans_to_the_end(&gw, &alice, &bob).await;

    let one = |column: &str| {
        gw.text(&format!(
            "SELECT coalesce({column}::text, '<null>') FROM game_record WHERE game_id = '{game}'"
        ))
    };
    assert_eq!(one("build"), baylee_build::short());
    assert_eq!(one("checked"), "true");
    assert_eq!(one("allow_spectators"), "true", "the default of a room");
    assert_eq!(one("winners"), "{1}", "Alice conceded, Bob won");
    assert!(one("turn").parse::<i32>().is_ok_and(|t| t >= 1));
    assert!(!one("end_reason").is_empty() && one("end_reason") != "<null>");
    assert_eq!(one("complete"), "true");

    let seat = |n: u8, column: &str| {
        gw.text(&format!(
            "SELECT coalesce({column}::text, '<null>') FROM game_record_seat \
             WHERE game_id = '{game}' AND seat = {n}"
        ))
    };
    assert_eq!(seat(0, "seated"), "person");
    assert_eq!(seat(1, "seated"), "person");
    assert_eq!(seat(0, "colours"), "U");
    assert_eq!(seat(1, "colours"), "B");
    assert_eq!(seat(0, "commander"), "<null>", "no commander in this deck");
    assert_eq!(seat(1, "commander"), "<null>");

    // A game against the house, at a table that allows no spectators: the
    // chair that is the house's says so, and the setting is the table's.
    let rhea = login(port, "rhea", "RheaCols");
    let ai = a_finished_game(&gw, &rhea, ",\"allow_spectators\":false").await;
    assert_eq!(
        gw.text(&format!(
            "SELECT allow_spectators::text FROM game_record WHERE game_id = '{ai}'"
        )),
        "false"
    );
    assert_eq!(
        gw.text(&format!(
            "SELECT string_agg(seated, ',' ORDER BY seat) FROM game_record_seat \
             WHERE game_id = '{ai}'"
        )),
        "person,house"
    );
    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM game_record_seat \
             WHERE game_id = '{ai}' AND colours IS NOT NULL"
        )),
        2,
        "both decks have a colour identity, even if it is none"
    );
    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM game_record_seat \
             WHERE game_id = '{ai}' AND account_id IS NOT NULL"
        )),
        1
    );
    // Whoever lost to the house is listed as having done so.
    let page = list(port, &rhea, "");
    let seats = page["games"][0]["seats"].as_array().expect("seats").clone();
    assert_eq!(seats[0]["who"], "you");
    assert_eq!(seats[1]["who"], "house");
    assert_eq!(seats[1]["name"], "House AI");
    assert_eq!(page["games"][0]["result"], "lost");
}

// ---------------------------------------------- consent, archiving, sharing

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn the_list_says_which_seats_share_and_which_are_mine() {
    let gw = spawn_gateway("replays-shared-list");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let alice = login(port, "alice", "AliceShared");
    let bob = login(port, "bob", "BobShared");
    let human = two_humans_to_the_end(&gw, &alice, &bob).await;
    let house = a_finished_game(&gw, &alice, "").await;

    let seats_of = |page: &Value, game: &str| -> Vec<Value> {
        page["games"]
            .as_array()
            .expect("games")
            .iter()
            .find(|g| g["game"] == game)
            .unwrap_or_else(|| panic!("{game} is listed: {page}"))["seats"]
            .as_array()
            .expect("seats")
            .clone()
    };
    // (shared, mine) of each seat, as booleans: the keys are always there.
    let flags = |seats: &[Value]| -> Vec<(bool, bool)> {
        seats
            .iter()
            .map(|s| {
                (
                    s["shared"]
                        .as_bool()
                        .unwrap_or_else(|| panic!("shared: {s}")),
                    s["mine"].as_bool().unwrap_or_else(|| panic!("mine: {s}")),
                )
            })
            .collect()
    };

    // Nobody has said: everyone shares, and only the asker's seat is mine.
    let hers = list(port, &alice, "");
    assert_eq!(
        flags(&seats_of(&hers, &human)),
        [(true, true), (true, false)]
    );
    let his = list(port, &bob, "");
    assert_eq!(
        flags(&seats_of(&his, &human)),
        [(true, false), (true, true)]
    );
    // The house always shares; the asker's seat is still the one that is mine.
    let seats = seats_of(&hers, &house);
    assert_eq!(seats[1]["who"], "house");
    assert_eq!(flags(&seats), [(true, true), (true, false)]);

    // Bob unticks. Alice reads it in her list, Bob in his; the seat he
    // spoke for is his, and nothing else moves.
    set_shared(port, &bob, &human, false);
    let hers = list(port, &alice, "");
    assert_eq!(
        flags(&seats_of(&hers, &human)),
        [(true, true), (false, false)]
    );
    let his = list(port, &bob, "");
    assert_eq!(
        flags(&seats_of(&his, &human)),
        [(true, false), (false, true)]
    );
    assert_eq!(
        flags(&seats_of(&hers, &house)),
        [(true, true), (true, false)],
        "another game of Alice's is as it was"
    );

    // And back: the same row, written over.
    set_shared(port, &bob, &human, true);
    assert_eq!(
        flags(&seats_of(&list(port, &alice, ""), &human)),
        [(true, true), (true, false)]
    );
    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM game_record_share WHERE game_id = '{human}'"
        )),
        1
    );
    // Alice unticks her own: a person's `shared` is false for her too, and
    // `mine` still marks it.
    set_shared(port, &alice, &human, false);
    assert_eq!(
        flags(&seats_of(&list(port, &alice, ""), &human)),
        [(false, true), (true, false)]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one table of cases, read top to bottom
async fn a_seat_says_whether_it_shares_from_the_table_never_from_the_request() {
    let gw = spawn_gateway("replays-share-put");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let alice = login(port, "alice", "AliceShare");
    let bob = login(port, "bob", "BobShare");
    let carol = login(port, "carol", "CarolShare");
    let game = two_humans_to_the_end(&gw, &alice, &bob).await;
    let rows = |game: &str| {
        gw.scalar(&format!(
            "SELECT count(*) FROM game_record_share WHERE game_id = '{game}'"
        ))
    };

    // The answer names the seat the table gave the account, and what it said.
    let (status, body) = put_share(port, &bob, &game, r#"{"shared":false}"#);
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap(),
        json!({ "seat": 1, "shared": false })
    );
    let (status, body) = put_share(port, &alice, &game, r#"{"shared":true}"#);
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap(),
        json!({ "seat": 0, "shared": true })
    );
    assert_eq!(
        gw.text(&format!(
            "SELECT string_agg(seat || '=' || shared, ',' ORDER BY seat) \
             FROM game_record_share WHERE game_id = '{game}'"
        )),
        "0=true,1=false"
    );
    // Saying it again changes nothing but the word.
    assert_eq!(put_share(port, &bob, &game, r#"{"shared":false}"#).0, 200);
    assert_eq!(rows(&game), 2);

    // A seat is the table's: a request that names one, or anything else, is
    // not a sharing choice. Nothing is written for it.
    for body in [
        "",
        "{}",
        "[]",
        "true",
        r#"{"shared":1}"#,
        r#"{"shared":"true"}"#,
        r#"{"shared":null}"#,
        r#"{"shared":true,"seat":0}"#,
        r#"{"shared":false,"seat":1}"#,
        r#"{"shared":true,"account":"x"}"#,
        r#"{"seat":0}"#,
    ] {
        let (status, answer) = put_share(port, &bob, &game, body);
        assert_eq!(status, 400, "{body:?}: {answer}");
    }
    assert_eq!(rows(&game), 2);
    assert_eq!(
        gw.text(&format!(
            "SELECT shared::text FROM game_record_share WHERE game_id = '{game}' AND seat = 1"
        )),
        "false",
        "Bob's seat is as he left it"
    );

    // Somebody who sat nowhere: 403, and no row. Nobody signed in: 401.
    // A game with no record: 404.
    let (status, answer) = put_share(port, &carol, &game, r#"{"shared":false}"#);
    assert_eq!(status, 403, "{answer}");
    assert_eq!(rows(&game), 2);
    let (status, answer) = put_share(port, &alice, "no-such-game", r#"{"shared":false}"#);
    assert_eq!(status, 404, "{answer}");
    assert_eq!(rows("no-such-game"), 0);
    assert_eq!(
        http(
            port,
            "PUT",
            &format!("/replays/{game}/share"),
            None,
            r#"{"shared":false}"#
        )
        .0,
        401
    );

    // While the game is played: once its record has begun, a player who
    // is in it says (the end-of-game dialog does so with the table open).
    let id = account_id(&gw, "carol");
    let (live, seat_token) = start_ai_game(port, &carol, "");
    gw.sql(&format!(
        "INSERT INTO game_record (game_id, bytes, complete, build, checked) \
         VALUES ('{live}', 0, false, '{}', true)",
        baylee_build::short()
    ));
    gw.sql(&format!(
        "INSERT INTO game_record_seat (game_id, seat, account_id, seated) \
         VALUES ('{live}', 0, '{id}', 'person')"
    ));
    let (status, body) = put_share(port, &carol, &live, r#"{"shared":false}"#);
    assert_eq!(status, 200, "{body}");
    assert_eq!(rows(&live), 1);
    concede_and_wait(&gw, &live, &seat_token).await;
    // And the word given in the game stands after it.
    let page = list(port, &carol, "");
    assert_eq!(page["games"][0]["seats"][0]["shared"], false, "{page}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn a_seat_that_unticks_shows_the_other_player_no_card_of_its_hand() {
    let gw = spawn_gateway("replays-mask");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let alice = login(port, "alice", "AliceMask");
    let bob = login(port, "bob", "BobMask");
    let game = two_humans_to_the_end(&gw, &alice, &bob).await;
    let bob_sees_his = || tape_of(port, &bob, &game, SEAT);
    let alice_asks_bob =
        |token: &str| tape_of(port, token, &game, r#"{"perspective":"seat","seat":1}"#);

    // The control: Bob's own tape, his hand in it. His deck is all Swamp,
    // hers all Island, so a card names whose it is.
    let bobs = bob_sees_his();
    let bob_hand = hand_ids_of(&bobs);
    assert!(bob_hand.len() >= 7, "a hand was shown: {bob_hand:?}");
    assert!(bobs.raw.contains("Swamp"));
    assert!(bobs.header.own && bobs.header.hands.is_empty());

    // Shared, as it is until he says otherwise: Alice drawn from his seat
    // is his tape, view for view.
    let seen = alice_asks_bob(&alice);
    assert_eq!(seen.header, bobs.header);
    assert_eq!(view_texts(&seen), view_texts(&bobs));
    assert_eq!(hand_ids_of(&seen), bob_hand);
    assert_eq!(seen.statics[0].your_seat, PlayerId::new(1));

    // Bob unticks. The same ask is now the public table drawn from his seat:
    // the header says his cards are not in it, and none of them is.
    set_shared(port, &bob, &game, false);
    let masked = alice_asks_bob(&alice);
    assert_eq!(masked.header.perspective, tape::Perspective::Seat);
    assert_eq!(masked.header.seat, 1);
    assert!(!masked.header.own);
    assert!(masked.header.hands.is_empty());
    assert_eq!(masked.stop.stop, StopKind::End);
    assert_eq!(masked.overs, 1);
    assert_eq!(masked.statics[0].your_seat, PlayerId::new(1));
    // As many views as his own tape: masking hides cards, never a moment.
    // `> 3` failed about one optimized run in ten (29.10.2026), because how
    // far the game gets before Alice concedes is a matter of timing.
    let views = masked.player_views();
    assert!(!views.is_empty());
    assert_eq!(views.len(), bobs.player_views().len());
    for view in &views {
        assert_eq!(view.seat, PlayerId::new(1));
        assert!(view.hand.is_empty(), "a hand in a masked tape");
        assert!(view.looking_at.is_empty());
        assert!(view.shared_hands.iter().all(|hand| hand.public));
        assert_eq!(
            view.seat(PlayerId::new(1)).map(|s| s.hand_count >= 7),
            Some(true),
            "the count of his hand is public"
        );
    }
    let named = ids_anywhere(&masked);
    assert!(
        named.is_disjoint(&bob_hand),
        "a card of Bob's hand is in the tape: {:?}",
        named.intersection(&bob_hand).collect::<Vec<_>>()
    );
    assert!(
        !masked.raw.contains("Swamp") && !masked.raw.contains("Island"),
        "a card of either hand is named in the tape"
    );
    assert!(
        !ids_anywhere(&bobs).is_disjoint(&bob_hand),
        "the control names the hand it is held against"
    );

    // Alice's own seat is always hers, whatever Bob said; Bob's own, too.
    let own = tape_of(port, &alice, &game, SEAT);
    assert!(own.header.own);
    let alice_hand = hand_ids_of(&own);
    assert!(alice_hand.len() >= 7 && alice_hand.is_disjoint(&bob_hand));
    assert!(own.raw.contains("Island") && !own.raw.contains("Swamp"));
    let mine = bob_sees_his();
    assert_eq!(mine.header, bobs.header);
    assert_eq!(view_texts(&mine), view_texts(&bobs), "Bob keeps his own");
    // Bob draws the table from Alice's seat: she has said nothing, so it is
    // her view, as hers is.
    let hers = tape_of(port, &bob, &game, r#"{"perspective":"seat","seat":0}"#);
    assert!(hers.header.own);
    assert_eq!(hand_ids_of(&hers), alice_hand);
    assert_eq!(view_texts(&hers), view_texts(&own));

    // The public tape is the same whatever was ticked.
    let public = tape_of(port, &alice, &game, r#"{"perspective":"public"}"#);
    assert!(!public.raw.contains("Swamp"));
    assert!(ids_anywhere(&public).is_disjoint(&bob_hand));

    // He ticks it again: the same ask is his full view once more.
    set_shared(port, &bob, &game, true);
    let back = alice_asks_bob(&alice);
    assert_eq!(back.header, bobs.header);
    assert_eq!(view_texts(&back), view_texts(&bobs));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn all_hands_shows_the_hands_that_are_shared_and_only_those() {
    let gw = spawn_gateway("replays-all-hands");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let alice = login(port, "alice", "AliceAll");
    let bob = login(port, "bob", "BobAll");
    let game = two_humans_to_the_end(&gw, &alice, &bob).await;
    let all = |token: &str, seat: Option<u8>| {
        let body = seat.map_or_else(
            || r#"{"perspective":"all_hands"}"#.to_owned(),
            |s| format!(r#"{{"perspective":"all_hands","seat":{s}}}"#),
        );
        tape_of(port, token, &game, &body)
    };
    let bob_hand = hand_ids_of(&tape_of(port, &bob, &game, SEAT));
    assert!(bob_hand.len() >= 7);

    // Everyone shares: Alice's seat, and Bob's hand beside it as shared.
    let shared = all(&alice, None);
    assert_eq!(shared.header.perspective, tape::Perspective::AllHands);
    assert_eq!(shared.header.seat, 0);
    assert!(shared.header.own);
    assert_eq!(shared.header.hands, vec![1]);
    assert_eq!(shared.stop.stop, StopKind::End);
    let alice_hand = hand_ids_of(&shared);
    assert!(alice_hand.len() >= 7, "her own hand");
    assert_eq!(
        shown_hand_ids_of(&shared, 1),
        bob_hand,
        "his, as he holds it"
    );
    assert_eq!(players_shown(&shared), BTreeSet::from([1]));
    assert!(shared.raw.contains("Swamp") && shared.raw.contains("Island"));

    // Bob unticks: the same ask holds her own hand and none of his.
    set_shared(port, &bob, &game, false);
    let unshared = all(&alice, None);
    assert_eq!(unshared.header.perspective, tape::Perspective::AllHands);
    assert!(unshared.header.own);
    assert!(unshared.header.hands.is_empty(), "{:?}", unshared.header);
    assert_eq!(hand_ids_of(&unshared), alice_hand);
    assert!(players_shown(&unshared).is_empty());
    assert!(ids_anywhere(&unshared).is_disjoint(&bob_hand));
    assert!(!unshared.raw.contains("Swamp"));

    // Bob asks all hands: Alice has said nothing, so hers is shown beside
    // his own seat.
    let his = all(&bob, None);
    assert_eq!(his.header.seat, 1);
    assert!(his.header.own);
    assert_eq!(his.header.hands, vec![0]);
    assert_eq!(hand_ids_of(&his), bob_hand);
    assert_eq!(shown_hand_ids_of(&his, 0), alice_hand);

    // Alice unticks too. Bob's own seat is his and still full, with no hand
    // beside it; her hand is hers.
    set_shared(port, &alice, &game, false);
    let his = all(&bob, None);
    assert!(his.header.own && his.header.hands.is_empty());
    assert_eq!(hand_ids_of(&his), bob_hand);
    assert!(players_shown(&his).is_empty());
    assert!(!his.raw.contains("Island"));

    // Drawn from a seat that has not shared, the asker's own hand is still
    // the asker's to see: Bob draws the table from Alice's seat, masked, with
    // his own hand beside it; Alice does the same from his.
    let from_hers = all(&bob, Some(0));
    assert_eq!(from_hers.header.seat, 0);
    assert!(!from_hers.header.own, "Alice's own cards are not in it");
    assert_eq!(from_hers.header.hands, vec![1]);
    assert!(
        from_hers
            .player_views()
            .iter()
            .all(|v| v.hand.is_empty() && v.seat == PlayerId::new(0))
    );
    assert_eq!(shown_hand_ids_of(&from_hers, 1), bob_hand);
    assert!(!from_hers.raw.contains("Island"));
    let from_his = all(&alice, Some(1));
    assert_eq!(from_his.header.seat, 1);
    assert!(!from_his.header.own);
    assert_eq!(from_his.header.hands, vec![0]);
    assert_eq!(shown_hand_ids_of(&from_his, 0), alice_hand);
    assert!(from_his.player_views().iter().all(|v| v.hand.is_empty()));
    assert!(!from_his.raw.contains("Swamp"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn the_house_always_shares_its_cards() {
    let gw = spawn_gateway("replays-house-shares");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let rhea = login(port, "rhea", "RheaHouse");
    let game = a_finished_game(&gw, &rhea, "").await;

    // Rhea says her own is not shared: the house's is not hers to withhold.
    set_shared(port, &rhea, &game, false);
    let page = list(port, &rhea, "");
    let seats = page["games"][0]["seats"].as_array().expect("seats");
    assert_eq!(
        (seats[0]["shared"].clone(), seats[0]["mine"].clone()),
        (json!(false), json!(true))
    );
    assert_eq!(seats[1]["who"], "house");
    assert_eq!(
        (seats[1]["shared"].clone(), seats[1]["mine"].clone()),
        (json!(true), json!(false))
    );

    // The table drawn from the house's chair holds its hand.
    let house = tape_of(port, &rhea, &game, r#"{"perspective":"seat","seat":1}"#);
    assert_eq!(house.header.seat, 1);
    assert!(house.header.own, "an AI seat has nothing to withhold");
    assert!(
        house
            .player_views()
            .iter()
            .all(|v| v.seat == PlayerId::new(1))
    );
    assert!(
        !hand_ids_of(&house).is_empty(),
        "the house's hand is in its own tape"
    );

    // All hands from her own (unshared, but hers) seat: the house's beside it.
    let all = tape_of(port, &rhea, &game, r#"{"perspective":"all_hands"}"#);
    assert_eq!(all.header.seat, 0);
    assert!(all.header.own);
    assert_eq!(all.header.hands, vec![1]);
    assert_eq!(players_shown(&all), BTreeSet::from([1]));
    assert!(!shown_hand_ids_of(&all, 1).is_empty());
    assert_eq!(shown_hand_ids_of(&all, 1), hand_ids_of(&house));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn archiving_a_game_moves_it_between_one_accounts_lists_and_nothing_else() {
    let gw = spawn_gateway("replays-archive");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let alice = login(port, "alice", "AliceArc");
    let bob = login(port, "bob", "BobArc");
    let carol = login(port, "carol", "CarolArc");
    let game = two_humans_to_the_end(&gw, &alice, &bob).await;
    let other = a_finished_game(&gw, &alice, "").await;
    assert_eq!(list(port, &alice, "")["total"], 2);
    assert_eq!(list(port, &bob, "")["total"], 1);
    let records = || gw.scalar("SELECT count(*) FROM game_record");
    let seats = || gw.scalar("SELECT count(*) FROM game_record_seat");
    let (records_before, seats_before) = (records(), seats());

    // Every listed game says which list it is in; the archive starts empty.
    let page = list(port, &alice, "");
    assert!(
        page["games"]
            .as_array()
            .unwrap()
            .iter()
            .all(|g| g["archived"] == false)
    );
    let none = list(port, &alice, "?archived=true");
    assert_eq!(none, json!({ "games": [], "total": 0 }));
    assert_eq!(
        list(port, &alice, "?archived=false")["total"],
        2,
        "false is the default"
    );

    // 200 with the choice, and the game moves: off Alice's main list and its
    // total, onto her archived one, and nowhere else.
    archive(port, &alice, &game, true);
    let main = list(port, &alice, "");
    assert_eq!(main["total"], 1, "{main}");
    assert_eq!(game_ids(&main), [other.as_str()]);
    assert_eq!(main["games"][0]["archived"], false);
    assert_eq!(list(port, &alice, "?offset=1")["games"], json!([]));
    let archived = list(port, &alice, "?archived=true");
    assert_eq!(archived["total"], 1, "{archived}");
    assert_eq!(game_ids(&archived), [game.as_str()]);
    assert_eq!(archived["games"][0]["archived"], true);
    assert_eq!(archived["games"][0]["seat"], 0, "the row is as it was");
    assert_eq!(archived["games"][0]["seats"].as_array().unwrap().len(), 2);
    let his = list(port, &bob, "");
    assert_eq!(his["total"], 1, "Bob's list is as it was: {his}");
    assert_eq!(game_ids(&his), [game.as_str()]);
    assert_eq!(his["games"][0]["archived"], false);
    assert_eq!(list(port, &bob, "?archived=true")["total"], 0);
    assert_eq!(list(port, &carol, "")["total"], 0);
    assert_eq!(list(port, &carol, "?archived=true")["total"], 0);

    // The record, its seats and Alice's right to a tape of it stay.
    assert_eq!((records(), seats()), (records_before, seats_before));
    let kept = tape_of(port, &alice, &game, SEAT);
    assert_eq!(kept.header.seat, 0);
    assert_eq!(kept.stop.stop, StopKind::End);
    // Her word about her seat stands, and so does his about his; neither
    // moves the game between lists.
    set_shared(port, &alice, &game, false);
    set_shared(port, &bob, &game, false);
    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM game_record_share WHERE game_id = '{game}' AND NOT shared"
        )),
        2
    );
    assert_eq!(list(port, &alice, "")["total"], 1);
    assert_eq!(list(port, &alice, "?archived=true")["total"], 1);

    // Saying it again is the same; the row is one.
    archive(port, &alice, &game, true);
    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM replay_archived WHERE game_id = '{game}'"
        )),
        1
    );
    assert_eq!(list(port, &alice, "?archived=true")["total"], 1);

    // Bad choices: not a body, or not a boolean, or more than the choice.
    for body in [
        "",
        "{}",
        "[]",
        "true",
        r#"{"archived":1}"#,
        r#"{"archived":"true"}"#,
        r#"{"archived":null}"#,
        r#"{"archived":false,"seat":0}"#,
        r#"{"archived":true,"account":"x"}"#,
        r#"{"hidden":true}"#,
        r#"{"shared":true}"#,
    ] {
        let (status, answer) = put_archived(port, &bob, &game, body);
        assert_eq!(status, 400, "{body:?}: {answer}");
    }
    assert_eq!(list(port, &bob, "")["total"], 1, "nothing was archived");
    assert_eq!(
        gw.scalar("SELECT count(*) FROM replay_archived"),
        1,
        "only Alice's row"
    );

    // Somebody who sat nowhere: 403, and nothing is archived for her. A game
    // with no record: 404. No session: 401.
    let (status, body) = put_archived(port, &carol, &game, r#"{"archived":true}"#);
    assert_eq!(status, 403, "{body}");
    let (status, body) = put_archived(port, &carol, "no-such-game", r#"{"archived":true}"#);
    assert_eq!(status, 404, "{body}");
    assert_eq!(
        put_archived(port, &alice, "no-such-game", r#"{"archived":true}"#).0,
        404
    );
    assert_eq!(
        http(
            port,
            "PUT",
            &format!("/replays/{game}/archived"),
            None,
            r#"{"archived":true}"#
        )
        .0,
        401
    );
    assert_eq!(gw.scalar("SELECT count(*) FROM replay_archived"), 1);

    // There is no DELETE any more: it archives nothing.
    for token in [&alice, &bob] {
        let (status, _) = http(port, "DELETE", &format!("/replays/{game}"), Some(token), "");
        assert!(matches!(status, 404 | 405), "DELETE answered {status}");
    }
    assert_eq!(gw.scalar("SELECT count(*) FROM replay_archived"), 1);
    assert_eq!(list(port, &bob, "")["total"], 1);
    assert_eq!((records(), seats()), (records_before, seats_before));

    // Bob archives it too: his own row. Then Alice takes hers back.
    archive(port, &bob, &game, true);
    assert_eq!(list(port, &bob, "")["total"], 0);
    assert_eq!(
        game_ids(&list(port, &bob, "?archived=true")),
        [game.as_str()]
    );
    assert_eq!(gw.scalar("SELECT count(*) FROM replay_archived"), 2);
    archive(port, &alice, &game, false);
    archive(port, &alice, &game, false);
    assert_eq!(list(port, &alice, "")["total"], 2);
    assert_eq!(list(port, &alice, "?archived=true")["total"], 0);
    assert_eq!(
        list(port, &bob, "?archived=true")["total"],
        1,
        "Bob's stays"
    );
    assert_eq!(gw.scalar("SELECT count(*) FROM replay_archived"), 1);
    assert_eq!((records(), seats()), (records_before, seats_before));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn a_player_who_leaves_keeps_the_word_the_seat_gave() {
    let gw = spawn_gateway("replays-consent-stays");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let alice = login(port, "alice", "AliceStays");
    let bob = login(port, "bob", "BobLeaves");
    let said = two_humans_to_the_end(&gw, &alice, &bob).await;
    let silent = two_humans_to_the_end(&gw, &alice, &bob).await;
    assert_ne!(said, silent);
    let bob_id = account_id(&gw, "bob");

    // In one game Bob unticks, and archives it; in the other he says nothing.
    let bob_hand = hand_ids_of(&tape_of(port, &bob, &said, SEAT));
    let bob_hand_silent = hand_ids_of(&tape_of(port, &bob, &silent, SEAT));
    assert!(bob_hand.len() >= 7 && bob_hand_silent.len() >= 7);
    set_shared(port, &bob, &said, false);
    archive(port, &bob, &said, true);
    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM replay_archived WHERE account_id = '{bob_id}'"
        )),
        1
    );

    let (status, body) = http(port, "DELETE", "/account", Some(&bob), "{}");
    assert_eq!(status, 204, "{body}");

    // His consent row stays (it belongs to the record); his archive went with
    // him; the record and both seats are there.
    assert_eq!(
        gw.text(&format!(
            "SELECT shared::text FROM game_record_share WHERE game_id = '{said}' AND seat = 1"
        )),
        "false"
    );
    assert_eq!(
        gw.scalar("SELECT count(*) FROM replay_archived"),
        0,
        "an archive is its account's"
    );
    assert_eq!(gw.scalar("SELECT count(*) FROM game_record"), 2);

    // Alice reads his seat as a former player's, with his word.
    let page = list(port, &alice, "");
    for (game, shared) in [(&said, false), (&silent, true)] {
        let row = page["games"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["game"] == game.as_str())
            .unwrap();
        assert_eq!(row["seats"][1]["who"], "former", "{row}");
        assert_eq!(row["seats"][1]["shared"], shared, "{row}");
        assert_eq!(row["seats"][1]["mine"], false);
    }

    // And his word governs her view: unticked, a former player's hand is
    // not shown; never ticked or unticked, it is.
    let ask = r#"{"perspective":"seat","seat":1}"#;
    let masked = tape_of(port, &alice, &said, ask);
    assert!(!masked.header.own);
    assert!(ids_anywhere(&masked).is_disjoint(&bob_hand));
    assert!(masked.player_views().iter().all(|v| v.hand.is_empty()));
    let all = tape_of(port, &alice, &said, r#"{"perspective":"all_hands"}"#);
    assert!(
        all.header.own && all.header.hands.is_empty(),
        "{:?}",
        all.header
    );
    assert!(players_shown(&all).is_empty());
    assert!(ids_anywhere(&all).is_disjoint(&bob_hand));

    let full = tape_of(port, &alice, &silent, ask);
    assert!(full.header.own);
    assert_eq!(hand_ids_of(&full), bob_hand_silent);
    let all = tape_of(port, &alice, &silent, r#"{"perspective":"all_hands"}"#);
    assert_eq!(all.header.hands, vec![1]);
    assert_eq!(shown_hand_ids_of(&all, 1), bob_hand_silent);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn a_seat_that_never_said_is_shared_in_a_game_after_the_cutoff_and_not_before() {
    let gw = spawn_gateway("replays-cutoff");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let alice = login(port, "alice", "AliceCut");
    let bob = login(port, "bob", "BobCut");
    let rhea = login(port, "rhea", "RheaCut");
    let before = two_humans_to_the_end(&gw, &alice, &bob).await;
    let after = two_humans_to_the_end(&gw, &alice, &bob).await;
    let house = a_finished_game(&gw, &rhea, "").await;
    let move_before_the_cutoff = |game: &str| {
        gw.sql(&format!(
            "UPDATE game_record SET started_at = \
                 (SELECT at FROM replay_share_cutoff) - interval '1 day' \
             WHERE game_id = '{game}'"
        ));
    };
    // The migration stored one moment, when the gateway first connected: the
    // games just played began after it.
    assert_eq!(gw.scalar("SELECT count(*) FROM replay_share_cutoff"), 1);
    let flags = |token: &str, game: &str| -> Vec<(bool, bool)> {
        list(port, token, "?limit=50")["games"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["game"] == game)
            .unwrap_or_else(|| panic!("{game} is listed"))["seats"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| (s["shared"].as_bool().unwrap(), s["mine"].as_bool().unwrap()))
            .collect()
    };
    assert_eq!(flags(&alice, &before), [(true, true), (true, false)]);
    assert_eq!(flags(&alice, &after), [(true, true), (true, false)]);
    let bob_hand = hand_ids_of(&tape_of(port, &bob, &before, SEAT));
    assert!(bob_hand.len() >= 7);

    // Played under the terms that promised nobody the recording: nobody said,
    // so nobody shares. In the list, for both players.
    move_before_the_cutoff(&before);
    assert_eq!(flags(&alice, &before), [(false, true), (false, false)]);
    assert_eq!(flags(&bob, &before), [(false, false), (false, true)]);
    assert_eq!(
        flags(&alice, &after),
        [(true, true), (true, false)],
        "the game after it is as it was"
    );

    // Alice draws the table from Bob's seat: the public table, his cards not
    // in it. Her own seat is hers, said or not.
    let ask = r#"{"perspective":"seat","seat":1}"#;
    let masked = tape_of(port, &alice, &before, ask);
    assert!(!masked.header.own, "{:?}", masked.header);
    assert!(masked.player_views().iter().all(|v| v.hand.is_empty()));
    assert!(ids_anywhere(&masked).is_disjoint(&bob_hand));
    assert!(!masked.raw.contains("Swamp"));
    let own = tape_of(port, &alice, &before, SEAT);
    assert!(own.header.own);
    assert!(hand_ids_of(&own).len() >= 7);
    // All hands: nobody else's hand to add.
    let all = tape_of(port, &alice, &before, r#"{"perspective":"all_hands"}"#);
    assert!(
        all.header.own && all.header.hands.is_empty(),
        "{:?}",
        all.header
    );
    assert!(players_shown(&all).is_empty());

    // Bob says yes: an explicit word beats the default, for his seat only.
    set_shared(port, &bob, &before, true);
    assert_eq!(flags(&alice, &before), [(false, true), (true, false)]);
    let full = tape_of(port, &alice, &before, ask);
    assert!(full.header.own);
    assert_eq!(hand_ids_of(&full), bob_hand);
    assert!(full.raw.contains("Swamp"));

    // After the cutoff the same ask is a yes without a word, and Bob's no
    // takes it away.
    let shown = tape_of(port, &alice, &after, ask);
    assert!(shown.header.own);
    assert!(hand_ids_of(&shown).len() >= 7);
    set_shared(port, &bob, &after, false);
    assert_eq!(flags(&alice, &after), [(true, true), (false, false)]);

    // The house's seat is shared whatever the date.
    move_before_the_cutoff(&house);
    let seats = list(port, &rhea, "")["games"][0]["seats"].clone();
    assert_eq!(seats[0]["shared"], false, "the player's unsaid, before");
    assert_eq!(seats[1]["who"], "house");
    assert_eq!(seats[1]["shared"], true, "{seats}");
    let theirs = tape_of(port, &rhea, &house, ask);
    assert!(theirs.header.own);
    assert!(!hand_ids_of(&theirs).is_empty());
    let all = tape_of(port, &rhea, &house, r#"{"perspective":"all_hands"}"#);
    assert_eq!(all.header.hands, vec![1]);
}
