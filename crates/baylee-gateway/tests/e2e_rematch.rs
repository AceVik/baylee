//! Playing the same table again.
//!
//! Rematch is the one lobby route that reaches across two games: it reads a
//! game that is over and writes a room that has not started. Everything that
//! can go wrong with it is about *which* of the two a thing belongs to — a
//! seat token that named the finished game, a chair still carrying the ready
//! flag from it, or a second room opened because the first one's pointer was
//! never looked at. So these tests finish a real game over the real sockets
//! and then press the button twice.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1::{self, Envelope};
use common::{attach_agent, http, json_field, login, spawn_gateway};
use futures_util::{SinkExt, StreamExt};
use prost::Message;

/// A deck of one basic land, which is all a game that ends in a concession
/// needs from either seat.
fn deck(port: u16, token: &str, name: &str, card: &str) -> String {
    let body = format!("{{\"name\":\"{name}\",\"cards\":[\"60 {card}\"]}}");
    let (status, body) = http(port, "POST", "/decks", Some(token), &body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

fn press_rematch(port: u16, token: &str, game: &str) -> (u16, String) {
    http(
        port,
        "POST",
        &format!("/lobby/games/{game}/rematch"),
        Some(token),
        "{}",
    )
}

/// Presses the button until the gateway stops saying the game is still on.
///
/// The concession travels engine → gateway before the lobby row turns over,
/// and a test that pressed once would be racing that. A `409` is the honest
/// "not yet"; anything else is the failure this loop must not hide.
async fn rematch_when_over(port: u16, token: &str, game: &str) -> String {
    for _ in 0..100 {
        let (status, body) = press_rematch(port, token, game);
        if status == 200 {
            return body;
        }
        assert_eq!(status, 409, "the rematch was refused: {body}");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("the game never ended");
}

/// Opens a seat socket, waiting for the engine to attach behind it.
async fn seat_socket(port: u16, game: &str, token: &str) -> Socket {
    let url = format!("ws://127.0.0.1:{port}/games/{game}/ws?token={token}");
    for _ in 0..50 {
        if let Ok((stream, _)) = tokio_tungstenite::connect_async(&url).await {
            return stream;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("the seat socket never opened");
}

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Waits for the opening payload, which is the first thing any seat is sent.
async fn opening(ws: &mut Socket) -> baylee_view::GameStatic {
    for _ in 0..50 {
        let frame = tokio::time::timeout(std::time::Duration::from_secs(10), ws.next())
            .await
            .expect("no frame within 10s")
            .expect("stream open")
            .expect("frame ok");
        if !frame.is_binary() {
            continue;
        }
        if let Some(v1::envelope::Msg::GameStatic(msg)) = Envelope::decode(frame.into_data())
            .expect("decode envelope")
            .msg
        {
            return serde_json::from_slice(&msg.static_json).expect("static json");
        }
    }
    panic!("the opening payload never arrived");
}

async fn concede(ws: &mut Socket) {
    let answer = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: String::new(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(&PlayerAction::Concede).unwrap(),
        })),
    };
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(
        answer.encode_to_vec().into(),
    ))
    .await
    .expect("send the concession");
}

/// Two people who both press the button end up at *one* new table, and it
/// starts on the second press.
///
/// The two halves of the route are here: the first press opens the room, the
/// second finds it rather than opening another, and the chair each caller is
/// handed is their own. That the room then starts is the third — nobody
/// pressed ready and nobody pressed start, because pressing *play again* is
/// both of those statements and there was no other screen to make them on.
#[tokio::test]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn both_players_press_rematch_and_land_at_the_same_new_table() {
    let gw = spawn_gateway("rematch-hvh");
    let port = gw.port;
    let agent = attach_agent(&gw).await;

    let tokens = [
        login(port, "ann@example.com", "ann_rematch"),
        login(port, "ben@example.com", "ben_rematch"),
    ];
    let decks = [
        deck(port, &tokens[0], "ann's islands", "Island"),
        deck(port, &tokens[1], "ben's forests", "Forest"),
    ];

    let create = format!("{{\"deck_id\":\"{}\",\"mode\":\"open\"}}", decks[0]);
    let (status, body) = http(port, "POST", "/lobby/games", Some(&tokens[0]), &create);
    assert_eq!(status, 200, "create open game: {body}");
    let first_game = json_field(&body, "game_id").to_string();
    let seat_token_a = json_field(&body, "seat_token").to_string();

    let join = format!("{{\"deck_id\":\"{}\"}}", decks[1]);
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{first_game}/join"),
        Some(&tokens[1]),
        &join,
    );
    assert_eq!(status, 200, "join: {body}");
    let seat_token_b = json_field(&body, "seat_token").to_string();

    for token in &tokens {
        let (status, body) = http(
            port,
            "POST",
            &format!("/lobby/games/{first_game}/ready"),
            Some(token),
            "{}",
        );
        assert_eq!(status, 200, "ready: {body}");
    }
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{first_game}/start"),
        Some(&tokens[0]),
        "",
    );
    assert_eq!(status, 200, "start: {body}");

    let mut ws_a = seat_socket(port, &first_game, &seat_token_a).await;
    let mut ws_b = seat_socket(port, &first_game, &seat_token_b).await;
    opening(&mut ws_a).await;
    opening(&mut ws_b).await;
    concede(&mut ws_a).await;

    // Ann asks for another game. Nobody has started it: Ben has not answered.
    let body = rematch_when_over(port, &tokens[0], &first_game).await;
    let second_game = json_field(&body, "game_id").to_string();
    assert_ne!(second_game, first_game, "a rematch is a new table");
    assert!(
        body.contains("\"seat\":0"),
        "Ann did not keep her chair: {body}"
    );
    let rematch_token_a = json_field(&body, "seat_token").to_string();
    assert_ne!(
        rematch_token_a, seat_token_a,
        "a seat token names one game for the whole of its life"
    );

    // Ben sees the room in the lobby with his own chair already in it, which
    // is how he learns there is one at all: the finished table is not in the
    // listing to carry the news.
    let (status, body) = http(port, "GET", "/lobby/games", Some(&tokens[1]), "");
    assert_eq!(status, 200, "listing: {body}");
    assert!(
        body.contains(&second_game),
        "the rematch room is not in Ben's lobby: {body}"
    );
    assert!(
        !body.contains(&first_game),
        "the finished table is still listed: {body}"
    );

    // And pressing the button himself joins it rather than opening a second.
    let (status, body) = press_rematch(port, &tokens[1], &first_game);
    assert_eq!(status, 200, "Ben's rematch: {body}");
    assert_eq!(
        json_field(&body, "game_id"),
        second_game,
        "the second press opened a second room"
    );
    assert!(
        body.contains("\"seat\":1"),
        "Ben did not keep his chair: {body}"
    );
    let rematch_token_b = json_field(&body, "seat_token").to_string();

    // Both tickets play: the room started itself when the last chair said so.
    let mut ws_a = seat_socket(port, &second_game, &rematch_token_a).await;
    let mut ws_b = seat_socket(port, &second_game, &rematch_token_b).await;
    let statics_a = opening(&mut ws_a).await;
    let statics_b = opening(&mut ws_b).await;
    assert_eq!(statics_a.your_seat, baylee_core::ids::PlayerId::new(0));
    assert_eq!(statics_b.your_seat, baylee_core::ids::PlayerId::new(1));
    assert_eq!(
        statics_a.seat_name(baylee_core::ids::PlayerId::new(1)),
        "ben_rematch",
        "the same two people, in the same two chairs"
    );

    agent.abort();
}

/// A table whose other chair is the house needs one press, because there is
/// nobody else to wait for.
///
/// This is the case the auto-start is for. An AI chair is ready the moment it
/// is configured, so the player's own press is the last one — and a rematch
/// that dropped them in a lobby to press two more buttons would be a worse
/// answer than the one-tap game they started with.
#[tokio::test]
async fn one_press_is_enough_when_the_other_chair_is_the_house() {
    let gw = spawn_gateway("rematch-ai");
    let port = gw.port;
    let agent = attach_agent(&gw).await;

    let token = login(port, "cass@example.com", "cass_rematch");
    let deck_id = deck(port, &token, "cass's swamps", "Swamp");

    let create = format!("{{\"deck_id\":\"{deck_id}\",\"mode\":\"ai\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&token), &create);
    assert_eq!(status, 200, "create ai game: {body}");
    let first_game = json_field(&body, "game_id").to_string();
    let seat_token = json_field(&body, "seat_token").to_string();

    let mut ws = seat_socket(port, &first_game, &seat_token).await;
    opening(&mut ws).await;
    concede(&mut ws).await;

    let body = rematch_when_over(port, &token, &first_game).await;
    let second_game = json_field(&body, "game_id").to_string();
    let rematch_token = json_field(&body, "seat_token").to_string();

    let mut ws = seat_socket(port, &second_game, &rematch_token).await;
    let statics = opening(&mut ws).await;
    assert_eq!(statics.your_seat, baylee_core::ids::PlayerId::new(0));
    assert!(
        statics.seats.iter().any(|s| s.is_ai),
        "the house kept its chair: {:?}",
        statics.seats
    );

    agent.abort();
}

/// The room answers to its own id, for the player who already went back.
///
/// The two people pressing the button are looking at different things. The one
/// who just finished has the game's id in front of them; the one who left for
/// the lobby has the room's, because the finished game is in nobody's listing.
/// So both address the same rematch.
///
/// And a chair the room copied is *reserved*, not taken: it carries the
/// account and the deck, and no seat token, because a token can only be minted
/// into a reply to the player it belongs to. Pressing ready on it from the
/// lobby says the right thing and is not enough — a table that started on such
/// a chair would be one its player could not open a socket to.
#[tokio::test]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn the_room_answers_to_its_own_id_as_well() {
    let gw = spawn_gateway("rematch-ready");
    let port = gw.port;
    let agent = attach_agent(&gw).await;

    let tokens = [
        login(port, "fay@example.com", "fay_rematch"),
        login(port, "gus@example.com", "gus_rematch"),
    ];
    let decks = [
        deck(port, &tokens[0], "fay's islands", "Island"),
        deck(port, &tokens[1], "gus's forests", "Forest"),
    ];

    let create = format!("{{\"deck_id\":\"{}\",\"mode\":\"open\"}}", decks[0]);
    let (status, body) = http(port, "POST", "/lobby/games", Some(&tokens[0]), &create);
    assert_eq!(status, 200, "create open game: {body}");
    let first_game = json_field(&body, "game_id").to_string();
    let seat_token_a = json_field(&body, "seat_token").to_string();

    let join = format!("{{\"deck_id\":\"{}\"}}", decks[1]);
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{first_game}/join"),
        Some(&tokens[1]),
        &join,
    );
    assert_eq!(status, 200, "join: {body}");
    let seat_token_b = json_field(&body, "seat_token").to_string();

    for token in &tokens {
        let (status, body) = http(
            port,
            "POST",
            &format!("/lobby/games/{first_game}/ready"),
            Some(token),
            "{}",
        );
        assert_eq!(status, 200, "ready: {body}");
    }
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{first_game}/start"),
        Some(&tokens[0]),
        "",
    );
    assert_eq!(status, 200, "start: {body}");

    let mut ws_a = seat_socket(port, &first_game, &seat_token_a).await;
    let mut ws_b = seat_socket(port, &first_game, &seat_token_b).await;
    opening(&mut ws_a).await;
    opening(&mut ws_b).await;
    concede(&mut ws_a).await;

    // Fay presses the button and the room opens.
    let body = rematch_when_over(port, &tokens[0], &first_game).await;
    let second_game = json_field(&body, "game_id").to_string();
    let rematch_token_a = json_field(&body, "seat_token").to_string();

    // Gus says he is ready from the lobby, which is true and is not enough:
    // his chair has no ticket on it yet, so the room stays where it is.
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{second_game}/ready"),
        Some(&tokens[1]),
        "{\"ready\":true}",
    );
    assert_eq!(status, 200, "Gus readies his chair: {body}");
    let (status, body) = http(port, "GET", "/lobby/games", Some(&tokens[1]), "");
    assert_eq!(status, 200, "listing: {body}");
    assert!(
        body.contains("\"state\":\"waiting\""),
        "the room started on a chair with no ticket on it: {body}"
    );
    // And the row says which button it wants. Nothing else on it does: it is
    // a waiting table with his own chair in it, exactly like any other.
    assert!(
        body.contains("\"rematch\":true"),
        "the listing does not say the room is a rematch: {body}"
    );

    // Pressing the button on the room he *can* see hands him his ticket, and
    // that is the press the table has been waiting for.
    let (status, body) = press_rematch(port, &tokens[1], &second_game);
    assert_eq!(status, 200, "Gus's rematch: {body}");
    assert_eq!(
        json_field(&body, "game_id"),
        second_game,
        "the room's own id names the room"
    );
    let rematch_token_b = json_field(&body, "seat_token").to_string();

    let mut ws_a = seat_socket(port, &second_game, &rematch_token_a).await;
    let mut ws_b = seat_socket(port, &second_game, &rematch_token_b).await;
    let statics = opening(&mut ws_a).await;
    opening(&mut ws_b).await;
    assert_eq!(statics.your_seat, baylee_core::ids::PlayerId::new(0));
    assert_eq!(
        statics.seat_name(baylee_core::ids::PlayerId::new(1)),
        "gus_rematch"
    );

    agent.abort();
}

/// The button belongs to the people who were at the table.
///
/// A finished game's id is not a secret — it was in every seat's own lobby
/// listing while the game was waiting — so the check has to be about who sat
/// there, not about who knows the id.
#[tokio::test]
async fn a_stranger_cannot_rematch_someone_elses_table() {
    let gw = spawn_gateway("rematch-stranger");
    let port = gw.port;
    let agent = attach_agent(&gw).await;

    let token = login(port, "dee@example.com", "dee_rematch");
    let outsider = login(port, "eve@example.com", "eve_rematch");
    let deck_id = deck(port, &token, "dee's plains", "Plains");

    let create = format!("{{\"deck_id\":\"{deck_id}\",\"mode\":\"ai\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&token), &create);
    assert_eq!(status, 200, "create ai game: {body}");
    let game = json_field(&body, "game_id").to_string();
    let seat_token = json_field(&body, "seat_token").to_string();

    let mut ws = seat_socket(port, &game, &seat_token).await;
    opening(&mut ws).await;
    concede(&mut ws).await;

    // Wait for the table to be over by the owner's own successful press, so
    // the outsider's refusal cannot be the "not over yet" answer in disguise.
    rematch_when_over(port, &token, &game).await;
    let (status, body) = press_rematch(port, &outsider, &game);
    assert_eq!(status, 403, "an outsider was let in: {body}");

    agent.abort();
}
