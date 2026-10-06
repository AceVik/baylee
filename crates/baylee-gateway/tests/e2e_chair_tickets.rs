//! A host's chair for a seat bridge (`docs/protocol.md` §"A host's chair
//! for a seat bridge"): the host asks for a chair ticket for one open chair
//! of its room, and a seat bridge that redeems it sits there on the host's
//! word, with no account of its own. So a language model gets to a table
//! where the gateway takes no guests, is full of them, or is a closed beta.
//!
//! Each test runs its own gateway on its own schema; the engine is the
//! real one, in process.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_protocol::v1::{self, Envelope};
use baylee_protocol::{TICKET_REFUSED, chair_redeem_path, chair_ticket_path};
use common::{Gateway, Socket, attach_agent, http, json_field, login, spawn_gateway_with};
use futures_util::SinkExt;
use prost::Message;

/// What a bridge sits down with.
const REDEEM: &str = r#"{"display_name":"LLM-test","deck":{"name":"swamps","cards":["60 Swamp"]}}"#;

fn deck(port: u16, token: &str) -> String {
    let body = r#"{"name":"forests","cards":["60 Forest"]}"#;
    let (status, body) = http(port, "POST", "/decks", Some(token), body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

/// A room of `seats` chairs `host` opened and sits in (chair 0).
fn room(port: u16, host: &str, seats: usize) -> String {
    let deck_id = deck(port, host);
    let create = format!("{{\"deck_id\":\"{deck_id}\",\"seats\":{seats},\"name\":\"bridges\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(host), &create);
    assert_eq!(status, 200, "create room: {body}");
    json_field(&body, "game_id").to_string()
}

/// `POST …/chairs/{seat}/ticket` as `session`: status and body.
fn mint(port: u16, session: &str, game: &str, seat: u32) -> (u16, String) {
    http(
        port,
        "POST",
        &chair_ticket_path(game, seat),
        Some(session),
        "{}",
    )
}

/// A ticket `host` was handed for `seat`.
fn ticket(port: u16, host: &str, game: &str, seat: u32) -> String {
    let (status, body) = mint(port, host, game, seat);
    assert_eq!(status, 200, "a chair ticket: {body}");
    json_field(&body, "ticket").to_string()
}

/// `POST …/chairs/{seat}/redeem` with `ticket`: status and body.
fn redeem(port: u16, ticket: &str, game: &str, seat: u32) -> (u16, String) {
    http(
        port,
        "POST",
        &chair_redeem_path(game, seat),
        Some(ticket),
        REDEEM,
    )
}

/// The listing's row of `game`, as `viewer` sees it.
fn row(port: u16, viewer: &str, game: &str) -> serde_json::Value {
    let (status, body) = http(port, "GET", "/lobby/games", Some(viewer), "");
    assert_eq!(status, 200, "{body}");
    let page: serde_json::Value = serde_json::from_str(&body).unwrap();
    page["games"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == game)
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

fn refused(status: u16, body: &str) {
    assert_eq!(status, 401, "{body}");
    assert!(body.contains(TICKET_REFUSED), "{body}");
}

/// Opens a seat and says ready once the table is shown; the socket and
/// the table's roster.
async fn sit(port: u16, game: &str, seat_token: &str) -> (Socket, serde_json::Value) {
    let mut ws = common::dial_seat(port, game, seat_token).await;
    let statics = tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(&mut ws).await {
                Some(v1::envelope::Msg::GameStatic(statics)) => {
                    common::send(
                        &mut ws,
                        &Envelope {
                            msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
                        },
                    )
                    .await;
                    break serde_json::from_slice::<serde_json::Value>(&statics.static_json)
                        .unwrap();
                }
                Some(_) => {}
                None => panic!("the socket closed before the table was shown"),
            }
        }
    })
    .await
    .expect("the table was shown");
    (ws, statics)
}

/// Waits for the curtain on `ws`.
async fn curtain(ws: &mut Socket) {
    tokio::time::timeout(common::WAIT_BUDGET, async {
        loop {
            match common::next_msg(ws).await {
                Some(v1::envelope::Msg::Curtain(_)) => break,
                Some(_) => {}
                None => panic!("the socket closed before play was allowed"),
            }
        }
    })
    .await
    .expect("the table opened");
}

fn me(port: u16, session: &str) -> String {
    let (status, body) = http(port, "GET", "/me", Some(session), "");
    assert_eq!(status, 200, "{body}");
    json_field(&body, "id").to_string()
}

/// `baylee-gateway invite create` against this gateway's schema: one key.
fn invite_key(gw: &Gateway) -> String {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
        .args(["invite", "create"])
        .env("DATABASE_URL", gw.database_url())
        .output()
        .expect("the command runs");
    assert!(out.status.success(), "{out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Waits until the game's record is complete in the store.
async fn stored(gw: &Gateway, game: &str) {
    for _ in 0..common::WAIT_TRIES {
        let done = gw.scalar(&format!(
            "SELECT count(*) FROM game_record WHERE game_id = '{game}' AND complete"
        ));
        if done == 1 {
            return;
        }
        tokio::time::sleep(common::WAIT_STEP).await;
    }
    panic!("the game's record was never completed");
}

/// The owner's case (06.10.2026): a closed beta that takes no guests. The
/// host's bridge sits on the host's ticket all the same, the room shows it
/// as the host's, it is ready the moment it sits, the table calls it what
/// it said it is, and the record names the host as the one who answers for
/// it without saying the host played it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn a_hosts_bridge_sits_where_no_guest_may_and_the_record_names_the_host() {
    let gw = spawn_gateway_with(
        "chair_no_guests",
        &[
            ("BAYLEE_REGISTRATION", "invite".into()),
            ("BAYLEE_GUESTS", "off".into()),
        ],
    );
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let key = invite_key(&gw);
    let register = format!(
        r#"{{"username":"hosta","display_name":"Hosta","password":"a-very-fine-password","invite_key":"{key}"}}"#
    );
    assert_eq!(http(port, "POST", "/auth/register", None, &register).0, 200);
    let (status, body) = http(
        port,
        "POST",
        "/auth/login",
        None,
        r#"{"username":"hosta","password":"a-very-fine-password"}"#,
    );
    assert_eq!(status, 200, "{body}");
    let host = json_field(&body, "token").to_string();
    let host_id = me(port, &host);
    // The door a bridge used to need is shut.
    let (status, _) = http(
        port,
        "POST",
        "/auth/guest",
        None,
        r#"{"display_name":"LLM-test"}"#,
    );
    assert_eq!(status, 403);

    let game = room(port, &host, 2);
    let accounts = gw.scalar("SELECT count(*) FROM account");
    let (status, body) = mint(port, &host, &game, 1);
    assert_eq!(status, 200, "{body}");
    assert!(json_number(&body, "expires_in") > 0, "{body}");
    let (status, body) = redeem(port, json_field(&body, "ticket"), &game, 1);
    assert_eq!(status, 200, "redeem: {body}");
    assert_eq!(json_number(&body, "seat"), 1);
    let seat_token = json_field(&body, "seat_token").to_string();
    assert_eq!(
        gw.scalar("SELECT count(*) FROM account"),
        accounts,
        "no account was made"
    );

    let listed = row(port, &host, &game);
    let chair = &listed["seats"][1];
    assert_eq!(chair["taken"], true, "{listed}");
    assert_eq!(chair["player"], "LLM-test", "{listed}");
    assert_eq!(chair["delegated_by"], listed["host"], "{listed}");
    assert_eq!(chair["you"], false, "{listed}");
    assert_eq!(listed["seats"][0]["delegated_by"], serde_json::Value::Null);
    // Not ready until the bridge says its mind answered: the room waits.
    assert_eq!(chair["ready"], false, "{listed}");
    assert_eq!(listed["startable"], false, "{listed}");
    let (status, _) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/start"),
        Some(&host),
        "{}",
    );
    assert_eq!(status, 409, "not everyone is ready");
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/chair/ready"),
        Some(&seat_token),
        "{}",
    );
    assert_eq!(status, 204, "{body}");
    // The host rearranging the table takes a person's yes back, and not the
    // bridge's: that says the model can play, not that it likes the table.
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/seats/0"),
        Some(&host),
        r#"{"team":1}"#,
    );
    assert_eq!(status, 200, "{body}");
    let listed = row(port, &host, &game);
    assert_eq!(listed["seats"][1]["ready"], true, "{listed}");
    assert_eq!(listed["startable"], true, "{listed}");

    // The bridge asks after its chair with its seat token.
    let (status, body) = http(
        port,
        "GET",
        &format!("/lobby/games/{game}/chair"),
        Some(&seat_token),
        "",
    );
    assert_eq!(status, 200, "{body}");
    assert_eq!(json_field(&body, "state"), "waiting");

    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/start"),
        Some(&host),
        "{}",
    );
    assert_eq!(status, 200, "start: {body}");
    let host_seat = {
        let (status, body) = http(
            port,
            "POST",
            &format!("/lobby/games/{game}/seat"),
            Some(&host),
            "{}",
        );
        assert_eq!(status, 200, "{body}");
        json_field(&body, "seat_token").to_string()
    };
    let (mut host_ws, statics) = sit(port, &game, &host_seat).await;
    let (_bridge_ws, _) = sit(port, &game, &seat_token).await;
    assert_eq!(statics["seats"][1]["display_name"], "LLM-test", "{statics}");
    curtain(&mut host_ws).await;
    let (status, body) = http(
        port,
        "GET",
        &format!("/lobby/games/{game}/chair"),
        Some(&seat_token),
        "",
    );
    assert_eq!(status, 200, "{body}");
    assert_eq!(json_field(&body, "state"), "playing");
    let concede = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: String::new(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(&baylee_engine::choice::PlayerAction::Concede).unwrap(),
        })),
    };
    host_ws
        .send(tokio_tungstenite::tungstenite::Message::Binary(
            concede.encode_to_vec().into(),
        ))
        .await
        .expect("concede");
    stored(&gw, &game).await;
    assert_eq!(
        gw.text(&format!(
            "SELECT coalesce(account_id::text, '-') || ' ' || coalesce(delegated_by::text, '-') \
             FROM game_record_seat WHERE game_id = '{game}' AND seat = 0"
        )),
        format!("{host_id} -"),
        "seat 0 is the host's own"
    );
    assert_eq!(
        gw.scalar(&format!(
            "SELECT count(*) FROM game_record_seat WHERE game_id = '{game}' AND seat = 1 \
             AND account_id IS NULL AND delegated_by = '{host_id}'"
        )),
        1,
        "seat 1 is the host's bridge's"
    );
}

fn json_number(body: &str, field: &str) -> i64 {
    common::json_number(body, field)
}

/// A ticket opens its own chair of its own room, once, and only for the
/// host: not another chair, not another room, not twice, not a chair
/// somebody took meanwhile, not the house's chair; and only a host signed
/// in to an account may ask for one.
#[tokio::test]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn a_chair_ticket_opens_its_own_chair_once_and_nothing_else() {
    let gw = spawn_gateway_with("chair_bound", &[]);
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let host = login(port, "hostb", "Hostb");
    let other = login(port, "otherb", "Otherb");
    let game = room(port, &host, 4);
    let elsewhere = room(port, &other, 3);

    // Another chair, then the right one: spent at the wrong door.
    let wrong_chair = ticket(port, &host, &game, 1);
    let (status, body) = redeem(port, &wrong_chair, &game, 2);
    refused(status, &body);
    let (status, body) = redeem(port, &wrong_chair, &game, 1);
    refused(status, &body);
    // Another room's chair of the same number.
    let wrong_room = ticket(port, &host, &game, 1);
    let (status, body) = redeem(port, &wrong_room, &elsewhere, 1);
    refused(status, &body);
    // No ticket, or one nobody made.
    let (status, body) = http(port, "POST", &chair_redeem_path(&game, 1), None, REDEEM);
    refused(status, &body);
    let (status, body) = redeem(port, &"0".repeat(64), &game, 1);
    refused(status, &body);
    // Once.
    let good = ticket(port, &host, &game, 1);
    assert_eq!(redeem(port, &good, &game, 1).0, 200);
    let (status, body) = redeem(port, &good, &game, 1);
    refused(status, &body);
    // Taken meanwhile: a person joined the chair the ticket was for.
    let late = ticket(port, &host, &game, 2);
    let other_deck = deck(port, &other);
    let join = format!("{{\"deck_id\":\"{other_deck}\",\"seat\":2}}");
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/join"),
        Some(&other),
        &join,
    );
    assert_eq!(status, 200, "{body}");
    assert_eq!(redeem(port, &late, &game, 2).0, 409);
    // Nor may a person join the chair a bridge sits in.
    let third = login(port, "thirdb", "Thirdb");
    let third_deck = deck(port, &third);
    let (status, _) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/join"),
        Some(&third),
        &format!("{{\"deck_id\":\"{third_deck}\",\"seat\":1}}"),
    );
    assert_eq!(status, 409, "the bridge's chair is taken");
    // Not a taken chair, the house's chair, a chair that is not there, or a
    // room that is not the caller's.
    assert_eq!(mint(port, &host, &game, 1).0, 409);
    let (status, _) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/seats/3"),
        Some(&host),
        r#"{"kind":"ai"}"#,
    );
    assert_eq!(status, 200);
    assert_eq!(mint(port, &host, &game, 3).0, 409);
    assert_eq!(mint(port, &host, &game, 9).0, 404);
    assert_eq!(mint(port, &other, &game, 3).0, 403);
    assert_eq!(mint(port, "not-a-session", &game, 3).0, 401);
    // A guest hosts its own room, and may not hand a chair of it on.
    let (status, body) = http(
        port,
        "POST",
        "/auth/guest",
        None,
        r#"{"display_name":"Visitor"}"#,
    );
    assert_eq!(status, 200, "{body}");
    let guest = json_field(&body, "token").to_string();
    let guests_room = room(port, &guest, 2);
    let (status, body) = mint(port, &guest, &guests_room, 1);
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("guest"), "{body}");
}

/// An unspent ticket dies at the end of its life.
#[tokio::test]
async fn a_chair_ticket_dies_when_its_life_is_over() {
    let gw = spawn_gateway_with("chair_expiry", &[("BAYLEE_CHAIR_TICKET_SECS", "1".into())]);
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let host = login(port, "hostc", "Hostc");
    let game = room(port, &host, 2);
    let (status, body) = mint(port, &host, &game, 1);
    assert_eq!(status, 200, "{body}");
    assert_eq!(json_number(&body, "expires_in"), 1);
    let expired = json_field(&body, "ticket").to_string();
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    let (status, body) = redeem(port, &expired, &game, 1);
    refused(status, &body);
    // A fresh one still seats.
    let fresh = ticket(port, &host, &game, 1);
    assert_eq!(redeem(port, &fresh, &game, 1).0, 200);
}

/// The host's word holds while it is at the table. Leaving the room, it
/// takes its unspent tickets and its bridges' chairs with it; a room that
/// closes takes everything; the host takes a chair back by arranging it;
/// and a bridge gives its own chair back with its seat token.
#[tokio::test]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn a_host_who_goes_takes_its_tickets_and_its_bridges_with_it() {
    let gw = spawn_gateway_with("chair_revoke", &[]);
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let host = login(port, "hostd", "Hostd");
    let player = login(port, "playerd", "Playerd");
    let game = room(port, &host, 4);
    let player_deck = deck(port, &player);
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/join"),
        Some(&player),
        &format!("{{\"deck_id\":\"{player_deck}\",\"seat\":3}}"),
    );
    assert_eq!(status, 200, "{body}");

    // A bridge gives its chair back, and its seat token opens nothing then.
    let (status, body) = redeem(port, &ticket(port, &host, &game, 1), &game, 1);
    assert_eq!(status, 200, "{body}");
    let gone = json_field(&body, "seat_token").to_string();
    let leave = format!("/lobby/games/{game}/chair/leave");
    assert_eq!(http(port, "POST", &leave, Some(&gone), "{}").0, 204);
    assert_eq!(row(port, &host, &game)["seats"][1]["taken"], false);
    assert_eq!(http(port, "POST", &leave, Some(&gone), "{}").0, 401);
    // A person's seat token is not a way out: they leave with their session.
    let (_, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/seat"),
        Some(&player),
        "{}",
    );
    let players_seat = json_field(&body, "seat_token").to_string();
    assert_eq!(http(port, "POST", &leave, Some(&players_seat), "{}").0, 403);
    let ready = format!("/lobby/games/{game}/chair/ready");
    assert_eq!(http(port, "POST", &ready, Some(&players_seat), "{}").0, 403);
    assert_eq!(http(port, "POST", &ready, Some(&gone), "{}").0, 401);

    // The host takes a chair back by arranging it.
    let (status, body) = redeem(port, &ticket(port, &host, &game, 1), &game, 1);
    assert_eq!(status, 200, "{body}");
    let kicked = json_field(&body, "seat_token").to_string();
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/seats/1"),
        Some(&host),
        r#"{"kind":"ai"}"#,
    );
    assert_eq!(status, 200, "kicked: {body}");
    let chair = format!("/lobby/games/{game}/chair");
    assert_eq!(http(port, "GET", &chair, Some(&kicked), "").0, 401);
    let (status, _) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/seats/1"),
        Some(&host),
        r#"{"kind":"human"}"#,
    );
    assert_eq!(status, 200);

    // The host goes: its unspent ticket and its bridge's chair go with it,
    // and the room passes to the player.
    let (status, body) = redeem(port, &ticket(port, &host, &game, 1), &game, 1);
    assert_eq!(status, 200, "{body}");
    let seated = json_field(&body, "seat_token").to_string();
    let unspent = ticket(port, &host, &game, 2);
    let (status, _) = http(
        port,
        "POST",
        &format!("/lobby/games/{game}/leave"),
        Some(&host),
        "{}",
    );
    assert_eq!(status, 204);
    let (status, body) = redeem(port, &unspent, &game, 2);
    refused(status, &body);
    assert_eq!(http(port, "GET", &chair, Some(&seated), "").0, 401);
    let listed = row(port, &player, &game);
    assert_eq!(listed["seats"][1]["taken"], false, "{listed}");
    assert_eq!(listed["yours"], true, "the room passed on: {listed}");

    // A room that closes takes everything: its host alone with a bridge
    // stands up, and the bridge's chair asks after a room that is over.
    let lone = room(port, &host, 3);
    let (status, body) = redeem(port, &ticket(port, &host, &lone, 1), &lone, 1);
    assert_eq!(status, 200, "{body}");
    let orphan = json_field(&body, "seat_token").to_string();
    let unspent = ticket(port, &host, &lone, 2);
    let (status, _) = http(
        port,
        "POST",
        &format!("/lobby/games/{lone}/leave"),
        Some(&host),
        "{}",
    );
    assert_eq!(status, 204);
    let (status, body) = redeem(port, &unspent, &lone, 2);
    refused(status, &body);
    let (status, body) = http(
        port,
        "GET",
        &format!("/lobby/games/{lone}/chair"),
        Some(&orphan),
        "",
    );
    assert!(
        status == 401 || (status == 200 && body.contains("\"over\"")),
        "{status} {body}"
    );
    assert_eq!(row(port, &host, &lone), serde_json::Value::Null);
}

/// Asking for chair tickets and redeeming them is bounded per host and
/// per address.
#[tokio::test]
async fn chair_tickets_run_into_the_limiter() {
    let gw = spawn_gateway_with("chair_limit", &[]);
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let host = login(port, "hoste", "Hoste");
    let game = room(port, &host, 2);
    let mut statuses = Vec::new();
    for _ in 0..31 {
        statuses.push(mint(port, &host, &game, 1).0);
    }
    assert!(statuses[..30].iter().all(|s| *s == 200), "{statuses:?}");
    assert_eq!(statuses[30], 429);
    // Another host has a budget of its own.
    let other = login(port, "othere", "Othere");
    let theirs = room(port, &other, 2);
    assert_eq!(mint(port, &other, &theirs, 1).0, 200);
    // Guessing at tickets from one address runs out too.
    let mut statuses = Vec::new();
    for n in 0..31 {
        statuses.push(redeem(port, &format!("{n:064}"), &game, 1).0);
    }
    assert!(statuses[..30].iter().all(|s| *s == 401), "{statuses:?}");
    assert_eq!(statuses[30], 429);
}

/// Neither a chair ticket nor the seat token it buys is written to the
/// log, refused or redeemed.
#[tokio::test]
async fn no_chair_ticket_reaches_the_log() {
    let gw = spawn_gateway_with("chair_logs", &[("RUST_LOG", "debug".into())]);
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let host = login(port, "hostf", "Hostf");
    let game = room(port, &host, 3);
    let spent = ticket(port, &host, &game, 2);
    let (status, body) = redeem(port, &spent, &game, 1);
    refused(status, &body);
    let good = ticket(port, &host, &game, 1);
    let (status, body) = redeem(port, &good, &game, 1);
    assert_eq!(status, 200, "{body}");
    let seat_token = json_field(&body, "seat_token").to_string();
    let logs = gw.logs();
    assert!(logs.contains("DEBUG"), "the log was at debug: {logs}");
    for secret in [&spent, &good, &seat_token] {
        assert!(!logs.contains(secret.as_str()), "a secret is in the log");
        assert!(
            !logs.contains(&secret[..16]),
            "a piece of one is in the log"
        );
    }
}

/// A gateway whose guest seats are all taken still seats a host's bridge:
/// it takes no guest seat.
#[tokio::test]
async fn a_full_guest_cap_still_seats_a_hosts_bridge() {
    let gw = spawn_gateway_with("chair_cap", &[("BAYLEE_GUEST_CAP", "1".into())]);
    let port = gw.port;
    let _agent = attach_agent(&gw).await;
    let guest = |name: &str| {
        http(
            port,
            "POST",
            "/auth/guest",
            None,
            &format!(r#"{{"display_name":"{name}"}}"#),
        )
        .0
    };
    assert_eq!(guest("First"), 200);
    assert_eq!(guest("Second"), 503, "the cap is reached");
    let host = login(port, "hostg", "Hostg");
    let game = room(port, &host, 2);
    let (status, body) = redeem(port, &ticket(port, &host, &game, 1), &game, 1);
    assert_eq!(status, 200, "{body}");
}
