//! End-to-end: two seat bridges play a whole game through a real gateway.
//!
//! A bridge is an ordinary socket player (`baylee-seat`): it signs in as a
//! guest under the name its mind discloses (`TEST-`, `HOUSE-`), stores a deck, takes a chair, says ready,
//! buys a ticket with its seat token and opens the seat socket like any
//! client. Here one bridge plays a script that answers every question with
//! the least answer it allows, and the other plays the house heuristic on
//! what its seat sees; every frame of the game crosses the gateway and the
//! engine an agent started.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use baylee_seat::bridge::{self, PlayOptions};
use baylee_seat::deck::Deck;
use baylee_seat::link::SeatLink;
use baylee_seat::lobby::{GuestSignIn, Lobby, Session, seat_name};
use baylee_seat::seat::Outcome;
use baylee_seat::{BridgeConfig, HouseMind, Mind, ScriptedMind, SeatCore, Transcript};
use common::{attach_agent_seeded, spawn_gateway};
use std::sync::Arc;
use std::time::Duration;

/// A whole game, with room to spare: the scripted seat plays nothing, so
/// the house has only its commander's life to get through.
const GAME_BUDGET: Duration = Duration::from_secs(600);

async fn guest(lobby: &Lobby, name: &str) -> Session {
    lobby
        .guest(&GuestSignIn {
            display_name: name.into(),
            invite_key: None,
        })
        .await
        .unwrap_or_else(|e| panic!("{name} signs in: {e:#}"))
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // e2e scenario script
async fn a_scripted_seat_and_a_house_seat_play_a_game_through_real_sockets() {
    let gateway = spawn_gateway("seat_bridge");
    // A fixed deal, not the one the gateway would draw. This game was
    // reported to fail under a loaded gate ("the house lost"), suspected of
    // the engine-server's decision clocks timing a seat out under
    // scheduling delay (`Deadline::Decide`/`StandIn`,
    // `EngineRunner::clocks`/`timeout`). That is ruled out: this harness's
    // `run_engine` (`tests/common/mod.rs`) never calls `clocks()` or
    // `timeout()` at all — it only fires the curtain's entrance deadline —
    // so no per-seat clock can ever expire here, loaded or not. Unseeded,
    // this test also played a different shuffle every run (turn counts from
    // 20 to 75 seen across five otherwise-identical local runs), which is a
    // real, independent source of a different outcome each time; the
    // reported failure was not reproduced locally (dozens of runs, seeded
    // and not, under artificial CPU load, all had the house win). Pinning
    // the deal at least takes chance out of the question: a recurrence
    // against this fixed seed would point at a real bug rather than an
    // unlucky draw, and this seed's game is the shortest tried (20 turns).
    let _agent = attach_agent_seeded(&gateway, 7).await;
    let lobby = Lobby::new(&format!("http://127.0.0.1:{}", gateway.port));

    // Each chair is called what its mind is, as `baylee-seat join` calls it.
    let scripted: Arc<dyn Mind> = Arc::new(ScriptedMind::idle());
    let house: Arc<dyn Mind> = Arc::new(HouseMind::default());
    let scripted_name = seat_name(scripted.disclosure(), "scripted").unwrap();
    let house_name = seat_name(house.disclosure(), "house").unwrap();
    assert_eq!(
        (scripted_name.as_str(), house_name.as_str()),
        ("TEST-scripted", "HOUSE-house")
    );
    let scripted_session = guest(&lobby, &scripted_name).await;
    let house_session = guest(&lobby, &house_name).await;
    let scripted_deck = Deck::acceptance("Allytifact").unwrap();
    let house_deck = Deck::acceptance("Victory").unwrap();
    let scripted_deck_id = lobby
        .upload(&scripted_session, &scripted_deck)
        .await
        .unwrap();
    let house_deck_id = lobby.upload(&house_session, &house_deck).await.unwrap();

    // The scripted seat opens the room and hosts it; the house takes the
    // other chair. Both say ready, and the host starts.
    let scripted_chair = lobby
        .open(&scripted_session, &scripted_deck_id, 2, "seat bridge")
        .await
        .unwrap();
    let game_id = scripted_chair.game_id.clone();
    let room = lobby.room(&house_session, &game_id).await.unwrap().unwrap();
    assert!(room.waiting() && room.has_a_free_chair(), "{room:?}");
    let house_chair = lobby
        .join(&house_session, &game_id, &house_deck_id, None, None)
        .await
        .unwrap();
    assert_eq!((scripted_chair.seat, house_chair.seat), (0, 1));
    lobby.ready(&scripted_session, &game_id).await.unwrap();
    lobby.ready(&house_session, &game_id).await.unwrap();
    lobby.start(&scripted_session, &game_id).await.unwrap();
    lobby
        .wait_for_start(&house_session, &game_id, Duration::from_millis(100))
        .await
        .unwrap();

    let seat_tokens = [
        scripted_chair.seat_token.clone(),
        house_chair.seat_token.clone(),
    ];
    let options = PlayOptions {
        min_think: Duration::ZERO,
        ..PlayOptions::default()
    };
    let core = |deck: &Deck, mind: &Arc<dyn Mind>| {
        SeatCore::new(
            BridgeConfig::default(),
            deck.list.clone(),
            mind.disclosure(),
        )
    };
    let mut scripted_link = SeatLink::new(lobby.clone(), scripted_chair, Some(scripted_session));
    let mut house_link = SeatLink::new(lobby.clone(), house_chair, Some(house_session));
    let mut scripted_transcript = Transcript::memory();
    let mut house_transcript = Transcript::memory();
    let (scripted_played, house_played) = tokio::time::timeout(GAME_BUDGET, async {
        tokio::join!(
            bridge::play(
                &mut scripted_link,
                core(&scripted_deck, &scripted),
                scripted,
                &mut scripted_transcript,
                &options,
            ),
            bridge::play(
                &mut house_link,
                core(&house_deck, &house),
                house,
                &mut house_transcript,
                &options,
            ),
        )
    })
    .await
    .unwrap_or_else(|_| panic!("the game did not end within {GAME_BUDGET:?}"));
    let scripted_played = scripted_played.unwrap_or_else(|e| panic!("the scripted seat: {e:#}"));
    let house_played = house_played.unwrap_or_else(|e| panic!("the house seat: {e:#}"));

    // One game, one result, seen the same from both chairs: the house won.
    assert!(house_played.result.is_some(), "{house_played:?}");
    assert_eq!(scripted_played.result, house_played.result);
    assert_eq!(
        house_played.stats.outcome,
        Some(Outcome::Won),
        "{house_played:?}"
    );
    assert_eq!(scripted_played.stats.outcome, Some(Outcome::Lost));

    for (played, name) in [(&scripted_played, "scripted"), (&house_played, "house")] {
        let stats = &played.stats;
        println!("{name}: {stats:?}");
        assert!(stats.turns > 1, "{name}: {stats:?}");
        assert!(
            stats.wakes > 0 && stats.answered.mind > 0,
            "{name}: {stats:?}"
        );
        assert!(
            stats.standing.total() > 0,
            "{name}: the orders answered nothing"
        );
        assert_eq!(stats.refused_by_table, 0, "{name}: {stats:?}");
        assert_eq!(stats.unanswerable, 0, "{name}: {stats:?}");
        assert_eq!(played.dials, 1, "{name}: the socket never dropped");
    }
    // The house as a mind never needs the house to answer for it.
    assert_eq!(house_played.stats.fallbacks.total(), 0, "{house_played:?}");

    // What a bridge writes down names no secret.
    for (transcript, name) in [
        (&scripted_transcript, "scripted"),
        (&house_transcript, "house"),
    ] {
        assert!(!transcript.lines().is_empty(), "{name} wrote nothing");
        for line in transcript.lines() {
            for token in &seat_tokens {
                assert!(
                    !line.contains(token.as_str()),
                    "{name} wrote a seat token: {line}"
                );
            }
        }
    }
}
