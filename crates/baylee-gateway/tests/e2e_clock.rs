//! A room chooses its clock, and the choice reaches the rules.
//!
//! Every game this gateway hosted ran 600 s to decide and 60 s to reconnect,
//! because nothing between a room and a `GamePreset` ever wrote `HouseRules`.
//! The wire was never the problem — the preset travels to the engine as JSON
//! with the house rules in it, and gamehost has always decoded them — so the
//! test that matters is not "does the field serialize" but **does what the
//! host picked arrive at the engine**.
//!
//! That is why this file stands where the engine stands: it captures the
//! `GameSetup` the gateway sends over `/engine/ws`, which is the last place
//! the choice can still be wrong and the first place nothing else can change
//! it.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{attach_agent_watching, http, json_field, json_number, login, spawn_gateway};

/// Opens a one-tap AI game with whatever clock body is given.
fn ai_game(port: u16, token: &str, deck: &str, extra: &str) -> (u16, String) {
    let body = format!("{{\"deck_id\":\"{deck}\",\"mode\":\"ai\"{extra}}}");
    http(port, "POST", "/lobby/games", Some(token), &body)
}

fn a_deck(port: u16, token: &str) -> String {
    let (status, body) = http(
        port,
        "POST",
        "/decks",
        Some(token),
        "{\"name\":\"d\",\"cards\":[\"60 Forest\"]}",
    );
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_clock_a_room_picks_is_the_clock_the_engine_is_given() {
    let gw = spawn_gateway("clock");
    let port = gw.port;
    let (agent, mut presets) = attach_agent_watching(&gw).await;
    let token = login(port, "clock@example.com", "clock_player");
    let deck = a_deck(port, &token);

    let (status, body) = ai_game(port, &token, &deck, ",\"clock\":\"blitz\"");
    assert_eq!(status, 200, "blitz game: {body}");

    // The assertion the whole ticket is about: not that the gateway stored a
    // number, but that the engine was handed it.
    let preset = tokio::time::timeout(common::WAIT_BUDGET, presets.recv())
        .await
        .expect("the engine was never set up")
        .expect("the agent stopped before a setup arrived");
    assert_eq!(
        preset.house_rules.decision_timeout_secs, 30,
        "the engine got the default clock, not the room's"
    );
    assert_eq!(preset.house_rules.reconnect_window_secs, 30);

    // And the parts of `HouseRules` nobody chose are still the defaults, so
    // picking a clock does not quietly re-decide the rest of the table.
    let fallback = baylee_core::preset::HouseRules::default();
    assert_eq!(
        preset.house_rules.mulligan_free_first,
        fallback.mulligan_free_first
    );
    assert_eq!(preset.house_rules.takebacks, fallback.takebacks);

    agent.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn saying_nothing_keeps_the_game_every_table_already_played() {
    let gw = spawn_gateway("clock-default");
    let port = gw.port;
    let (agent, mut presets) = attach_agent_watching(&gw).await;
    let token = login(port, "default@example.com", "default_player");
    let deck = a_deck(port, &token);

    let (status, body) = ai_game(port, &token, &deck, "");
    assert_eq!(status, 200, "default game: {body}");

    let preset = tokio::time::timeout(common::WAIT_BUDGET, presets.recv())
        .await
        .expect("the engine was never set up")
        .expect("the agent stopped before a setup arrived");
    // The counter-test for the one above: if this file could not tell the two
    // apart, the blitz assertion would pass against a gateway that ignored
    // the field entirely.
    assert_eq!(preset.house_rules.decision_timeout_secs, 600);
    assert_eq!(preset.house_rules.reconnect_window_secs, 60);

    agent.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_player_can_see_the_pace_before_sitting_down() {
    let gw = spawn_gateway("clock-listing");
    let port = gw.port;
    let host = login(port, "host@example.com", "clock_host");
    let deck = a_deck(port, &host);

    let body = format!("{{\"deck_id\":\"{deck}\",\"name\":\"fast table\",\"clock\":\"blitz\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&host), &body);
    assert_eq!(status, 200, "open room: {body}");

    // A room has no preset until it starts, so this number can only come from
    // the room itself — which is why the lobby model carries it rather than
    // reading it back off a preset that does not exist yet.
    let stranger = login(port, "stranger@example.com", "clock_stranger");
    let (status, listing) = http(port, "GET", "/lobby/games", Some(&stranger), "");
    assert_eq!(status, 200, "listing: {listing}");
    assert_eq!(
        json_number(&listing, "decide_secs"),
        30,
        "a player cannot see what pace this table plays at: {listing}"
    );
    assert_eq!(json_number(&listing, "reconnect_secs"), 30);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_clock_nobody_offers_is_refused_and_says_what_there_is() {
    let gw = spawn_gateway("clock-refused");
    let port = gw.port;
    let token = login(port, "bad@example.com", "bad_clock");
    let deck = a_deck(port, &token);

    // Rooms rather than `mode:"ai"`, and for a reason worth stating: opening
    // a room needs no agent, so these assertions are about the clock and
    // nothing else. Asked as one-tap AI games they would have to pass a
    // second gate — "is an engine available" — and a `503` from that gate
    // reads exactly like a refusal from this one.
    let room = |extra: &str| {
        let body = format!("{{\"deck_id\":\"{deck}\"{extra}}}");
        http(port, "POST", "/lobby/games", Some(&token), &body)
    };

    let (status, body) = room(",\"clock\":\"bullet\"");
    assert_eq!(status, 400, "an unknown clock: {body}");
    assert!(
        body.contains("blitz") && body.contains("casual"),
        "the refusal has to name the clocks there are: {body}"
    );

    let (status, body) = room(",\"reconnect_window_secs\":0");
    assert_eq!(status, 400, "a zero stand-in window: {body}");
    assert!(
        body.contains("reconnect_window_secs"),
        "the refusal has to name the field: {body}"
    );

    // Neither refusal left a room behind. Checked before the one that
    // succeeds, so an empty listing here means exactly that.
    let (status, listing) = http(port, "GET", "/lobby/games", Some(&token), "");
    assert_eq!(status, 200);
    assert!(
        !listing.contains("decide_secs"),
        "a refused room was opened anyway: {listing}"
    );

    // And the asymmetry that is the whole point: zero is a *choice* for the
    // decision clock and a refusal for the stand-in window. A table with no
    // decision clock is a legitimate game; a table that waits forever on a
    // player who closed their laptop is the failure the stand-in ended.
    let (status, body) = room(",\"decision_timeout_secs\":0");
    assert_eq!(
        status, 200,
        "no decision clock is a legitimate table: {body}"
    );
    let (status, listing) = http(port, "GET", "/lobby/games", Some(&token), "");
    assert_eq!(status, 200);
    assert_eq!(
        json_number(&listing, "decide_secs"),
        0,
        "an untimed room does not say so: {listing}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_gateway_publishes_the_clocks_it_will_accept() {
    let gw = spawn_gateway("clock-config");
    // Unauthenticated, like the rest of `/auth/config`: a client builds its
    // room screen before anybody has signed in.
    let (status, body) = http(gw.port, "GET", "/auth/config", None, "");
    assert_eq!(status, 200, "config: {body}");
    for name in ["casual", "standard", "blitz", "untimed"] {
        assert!(body.contains(name), "{name} is not offered: {body}");
    }
    // Every published clock has to be one the validator accepts, or the menu
    // and the door disagree. `clock::tests` proves that over the table
    // itself; this proves the table is what is published.
    assert!(
        body.contains("\"decide_secs\":600"),
        "the default clock is not in the menu: {body}"
    );
}

/// The room's clock reaches the *seat*, not only the engine — the whole
/// point of #98.
///
/// A client is disconnected for exactly the reconnect window it would be
/// counting, so join is the only moment it can be told; and three ways into
/// a game read no lobby row at all. This asserts the end of that chain: a
/// room picks `blitz`, and the first payload a seat's socket receives says
/// 30 and 30.
#[tokio::test(flavor = "multi_thread")]
async fn a_seat_is_told_the_two_limits_its_table_plays_at() {
    let gw = spawn_gateway("clock-static");
    let port = gw.port;
    let _agent = common::attach_agent(&gw).await;

    let token = login(port, "sheet@example.com", "clock_sheet");
    let deck = a_deck(port, &token);
    let (status, body) = ai_game(port, &token, &deck, ",\"clock\":\"blitz\"");
    assert_eq!(status, 200, "blitz game: {body}");
    let game_id = json_field(&body, "game_id").to_string();
    let seat_token = json_field(&body, "seat_token").to_string();

    let url = format!("ws://127.0.0.1:{port}/games/{game_id}/ws?token={seat_token}");
    let mut ws = common::dial_seat(&url).await;
    let frame = tokio::time::timeout(common::WAIT_BUDGET, {
        use futures_util::StreamExt as _;
        ws.next()
    })
    .await
    .expect("the table said nothing")
    .expect("the socket was closed")
    .expect("frame ok");
    let env = <baylee_protocol::v1::Envelope as prost::Message>::decode(frame.into_data())
        .expect("decode envelope");
    let Some(baylee_protocol::v1::envelope::Msg::GameStatic(msg)) = env.msg else {
        panic!("a seat's first frame is the opening payload");
    };
    let statics: baylee_view::GameStatic =
        serde_json::from_slice(&msg.static_json).expect("static json");

    assert_eq!(
        statics.decision_secs,
        Some(30),
        "the seat was not told how long it has to answer"
    );
    assert_eq!(
        statics.reconnect_secs,
        Some(30),
        "the seat was not told how long it may be gone"
    );
}
