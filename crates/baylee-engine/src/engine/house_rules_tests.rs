//! The deliberate departures from the Comprehensive Rules, and one rule
//! that reads like one.
//!
//! These are *house rules*, not bugs, and that is exactly why they need
//! tests: every one of them looks like a rules mistake to anyone reading the
//! engine against CR, so nothing but a test says "this is on purpose".
//!
//! 1. The first mulligan is free in a duel too (CR 103.5c makes it free
//!    only in a multiplayer or Brawl game; in a duel CR 103.5 charges for
//!    every one).
//! 2. Not a departure: with three or more players nobody skips their first
//!    draw step, which is CR 103.8c. Listed because the two-player skip
//!    (CR 103.8a) is the one a reader remembers.
//! 3. A real endless loop resolves once and is then broken (CR 104.4b makes
//!    it a draw) — covered in `loop_tests`, since it needs a loop to run.
//! 4. Every seat answers its mulligans at once, in any order (CR 103.5 has
//!    each player declare in turn order, and only then are the mulligans
//!    taken together); see `engine::mulligan`.

use super::testkit::{Duel, RegistryLookup, card_index};
use super::*;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
    SeatSpec,
};

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// A table of `seats` players, all AI-controlled, each on a plain deck, with
/// every mulligan still to be answered.
fn open_table(seats: usize, seed: u64) -> Engine<RegistryLookup> {
    let deck: Vec<DeckEntry> = (0..60)
        .map(|_| DeckEntry {
            card: forest(),
            print: PrintRef::new(0),
        })
        .collect();
    let preset = GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: (0..seats)
            .map(|_| SeatSpec {
                controller: SeatController::Ai(AIProfile::default()),
                capabilities: baylee_core::preset::SeatCapabilities::default(),
                deck: deck.clone(),
                sideboard: vec![],
                commanders: vec![],
                starting_life: None,
                starting_hand: None,
                starting_battlefield: vec![],
                emblems: vec![],
                team: None,
            })
            .collect(),
    };
    Engine::new(&preset, RegistryLookup).expect("table starts")
}

/// The same table once every seat has kept, in seat order.
fn table(seats: usize, seed: u64) -> Engine<RegistryLookup> {
    let mut engine = open_table(seats, seed);
    for _ in 0..seats {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected a mulligan, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    engine
}

/// Walks to the first main phase and reports how many cards each seat drew
/// on the way, by hand size.
fn hand_sizes(engine: &mut Engine<RegistryLookup>) -> Vec<usize> {
    for _ in 0..40 {
        if matches!(engine.state().turn.phase, Phase::FirstMain) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending before the first main: {other:?}"),
        }
    }
    engine
        .state()
        .players
        .iter()
        .map(|p| engine.state().zones.list(ZoneLocation::Hand(p.id)).len())
        .collect()
}

/// House rule 1: the first mulligan costs nothing, so a seat that
/// mulligans once keeps seven. Under CR 103.5 it would keep six.
#[test]
fn the_first_mulligan_is_free() {
    let mut engine = Duel::new(3, forest()).start();
    let Pending::Mulligan {
        player,
        taken,
        next_is_free,
    } = engine.pending().clone()
    else {
        panic!("expected a mulligan")
    };
    assert_eq!(taken, 0);
    assert!(next_is_free, "the first mulligan should be free");

    engine.apply(player, PlayerAction::MulliganTake).unwrap();
    let Pending::Mulligan {
        player,
        taken,
        next_is_free,
    } = engine.pending().clone()
    else {
        panic!("expected the next mulligan decision")
    };
    assert_eq!(taken, 1);
    assert!(!next_is_free, "only the first one is free");

    // Keeping now costs nothing: no cards go to the bottom.
    engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    assert!(
        !matches!(engine.pending(), Pending::MulliganBottom { .. }),
        "a free mulligan asked the seat to bottom a card"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(PlayerId::new(0)))
            .len(),
        7,
        "one mulligan should still leave seven cards"
    );
}

/// The second mulligan does cost a card, so the free one is a single
/// exemption rather than mulligans being free in general.
#[test]
fn the_second_mulligan_still_costs_a_card() {
    let mut engine = Duel::new(3, forest()).start();
    for _ in 0..2 {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected a mulligan")
        };
        engine.apply(player, PlayerAction::MulliganTake).unwrap();
    }
    let Pending::Mulligan { player, taken, .. } = engine.pending().clone() else {
        panic!("expected a mulligan")
    };
    assert_eq!(taken, 2);
    engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    match engine.pending() {
        Pending::MulliganBottom { count, .. } => assert_eq!(*count, 1),
        other => panic!("the second mulligan should cost one card, got {other:?}"),
    }
}

/// CR 103.8a as printed: in a two-player game the starting player skips
/// their first draw step.
#[test]
fn in_a_duel_the_starting_player_still_skips_the_draw() {
    let mut engine = table(2, 11);
    let hands = hand_sizes(&mut engine);
    assert_eq!(
        hands,
        vec![7, 7],
        "the starting player drew on turn 1 in a duel"
    );
}

/// Rule 2, CR 103.8c: with three or more players nobody skips, so the starting
/// player reaches their first main phase with eight cards. The skip exists
/// to blunt a duel's first-turn advantage, which does not apply at a
/// multiplayer table.
#[test]
fn at_a_multiplayer_table_the_starting_player_draws() {
    for seats in [3usize, 4] {
        let mut engine = table(seats, 21);
        let hands = hand_sizes(&mut engine);
        assert_eq!(
            hands[0], 8,
            "the starting player did not draw at a {seats}-seat table"
        );
        for (seat, size) in hands.iter().enumerate().skip(1) {
            assert_eq!(*size, 7, "seat {seat} drew before its own turn");
        }
    }
}

/// A concession during the mulligans takes the conceder out and leaves every
/// other seat's mulligan to be decided (CR 104.3a, CR 103.5); nobody who has
/// left is given priority (CR 800.4a, CR 800.4j).
#[test]
fn a_concession_during_the_mulligans_leaves_the_others_to_decide() {
    for conceder in [2_u8, 0] {
        let mut engine = Duel::table(257, forest(), 3).start();
        engine
            .apply(PlayerId::new(conceder), PlayerAction::Concede)
            .unwrap();
        let mut asked = Vec::new();
        while let Pending::Mulligan { player, .. } = engine.pending().clone() {
            asked.push(player.get());
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
        }
        let others: Vec<u8> = (0..3).filter(|p| *p != conceder).collect();
        assert_eq!(
            asked,
            others,
            "seat {conceder} conceded; then {:?}",
            engine.pending()
        );
        assert!(
            !matches!(engine.pending(), Pending::Priority { player, .. } if player.get() == conceder),
            "seat {conceder} left the game and holds priority"
        );
    }
}

/// A concession by the starting player during the mulligans hands turn 1 to
/// the next seat: a player who has left does not begin a turn (CR 800.4k).
#[test]
fn when_the_starting_player_concedes_during_the_mulligans_turn_1_is_the_next_seats() {
    let mut engine = open_table(3, 257);
    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    engine.apply(seat(2), PlayerAction::MulliganKeep).unwrap();
    engine.apply(seat(1), PlayerAction::MulliganKeep).unwrap();
    assert_eq!(turns_started(&engine), vec![(1, 1)]);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat(1)),
        "{:?}",
        engine.pending()
    );
}

/// In a duel, a concession during the mulligans ends the game there: the
/// other seat is not asked to keep a hand for a game it has already won.
#[test]
fn a_concession_during_the_mulligans_of_a_duel_ends_the_game() {
    for (conceder, kept_first) in [(1, false), (0, true)] {
        let mut engine = open_table(2, 257);
        if kept_first {
            engine
                .apply(seat(conceder), PlayerAction::MulliganKeep)
                .unwrap();
        }
        engine.apply(seat(conceder), PlayerAction::Concede).unwrap();
        let Pending::GameOver(result) = engine.pending() else {
            panic!("seat {conceder} conceded; then {:?}", engine.pending())
        };
        assert_eq!(
            result.winner,
            Some(crate::win::Victor::Player(seat(1 - conceder)))
        );
        assert!(engine.awaited().is_empty());
        assert!(
            turns_started(&engine).is_empty(),
            "a turn began after the game ended"
        );
    }
}

/// House rule 4: every seat is asked its mulligan from the start, and each
/// answers its own in whatever order the answers come.
#[test]
fn every_seat_answers_its_own_mulligan_in_any_order() {
    let mut engine = open_table(3, 257);
    assert_eq!(engine.awaited(), [0, 1, 2].map(seat).into_iter().collect());
    for p in 0..3 {
        assert!(matches!(
            engine.pending_for(seat(p)),
            Some(Pending::Mulligan { player, taken: 0, .. }) if *player == seat(p)
        ));
    }

    // The last seat keeps first; the first is still the one `pending` shows.
    engine.apply(seat(2), PlayerAction::MulliganKeep).unwrap();
    assert_eq!(engine.awaited(), [0, 1].map(seat).into_iter().collect());
    assert!(engine.pending_for(seat(2)).is_none());
    assert!(matches!(engine.pending(), Pending::Mulligan { player, .. } if *player == seat(0)));

    // Seat 1 takes twice while seat 0 has not said anything.
    let dealt = hand(&engine, 1);
    for _ in 0..2 {
        engine.apply(seat(1), PlayerAction::MulliganTake).unwrap();
    }
    assert_ne!(hand(&engine, 1), dealt, "a mulligan dealt the same hand");
    assert_eq!(hand(&engine, 1).len(), 7);
    assert!(matches!(
        engine.pending_for(seat(1)),
        Some(Pending::Mulligan { taken: 2, .. })
    ));
    assert!(matches!(
        engine.pending_for(seat(0)),
        Some(Pending::Mulligan { taken: 0, .. })
    ));

    // Both owe a card at once, and the later seat answers first.
    engine.apply(seat(1), PlayerAction::MulliganKeep).unwrap();
    for _ in 0..2 {
        engine.apply(seat(0), PlayerAction::MulliganTake).unwrap();
    }
    engine.apply(seat(0), PlayerAction::MulliganKeep).unwrap();
    for p in [0, 1] {
        assert!(matches!(
            engine.pending_for(seat(p)),
            Some(Pending::MulliganBottom { count: 1, .. })
        ));
    }
    bottom(&mut engine, 1, 1);
    assert!(
        turns_started(&engine).is_empty(),
        "turn 1 began with seat 0 still owing a card"
    );
    assert!(
        matches!(engine.pending(), Pending::MulliganBottom { player, .. } if *player == seat(0))
    );

    // The last answer begins turn 1, once.
    bottom(&mut engine, 0, 1);
    assert_eq!(turns_started(&engine), vec![(1, 0)]);
    assert_eq!(engine.awaited(), [seat(0)].into_iter().collect());
    assert!(matches!(
        engine.pending_for(seat(0)),
        Some(Pending::Priority { .. })
    ));
    assert!(engine.pending_for(seat(1)).is_none());
}

/// Before turn 1 a seat can answer only its own mulligan question, concede,
/// or change an automation setting, and nothing it is refused moves anything.
#[test]
fn before_turn_1_a_seat_answers_only_its_own_mulligan() {
    let mut engine = open_table(3, 257);
    engine.apply(seat(0), PlayerAction::MulliganKeep).unwrap();
    let before = engine.snapshot_hash();
    let refused = [
        (0, PlayerAction::MulliganKeep),
        (0, PlayerAction::MulliganTake),
        (1, PlayerAction::PassPriority),
        (1, PlayerAction::OfferDraw),
        (
            1,
            PlayerAction::ChooseObjects {
                objects: hand(&engine, 1)[..1].to_vec(),
            },
        ),
    ];
    for (p, action) in refused {
        assert!(
            engine.apply(seat(p), action.clone()).is_err(),
            "seat {p} was allowed {action:?} before turn 1"
        );
    }
    assert_eq!(
        engine.snapshot_hash(),
        before,
        "a refused answer moved the game"
    );
    engine
        .apply(
            seat(1),
            PlayerAction::SetPriorityHold(crate::choice::PriorityHold::PassWhenNothingToDo),
        )
        .unwrap();
    assert_eq!(engine.awaited(), [1, 2].map(seat).into_iter().collect());
    assert!(turns_started(&engine).is_empty());
}

/// Nothing runs before turn 1 whatever the answer flag says: #267 was the
/// flag cleared by a concession and the game run past every open mulligan.
#[test]
fn the_machine_does_not_run_while_the_mulligans_are_open() {
    let mut engine = open_table(3, 257);
    engine.awaiting_answer = false;
    engine.run_until_choice();
    assert!(
        matches!(engine.pending(), Pending::Mulligan { player, .. } if *player == seat(0)),
        "{:?}",
        engine.pending()
    );
    assert_eq!(engine.state().turn.step, Step::Untap);
    assert_eq!(engine.awaited(), [0, 1, 2].map(seat).into_iter().collect());
}

/// Who is still deciding is part of the game's state: a keep moves nothing
/// on the board and still leaves a different game behind it.
#[test]
fn the_open_mulligans_are_in_the_snapshot_hash() {
    let asked = open_table(3, 257);
    let mut kept = open_table(3, 257);
    assert_eq!(asked.snapshot_hash(), kept.snapshot_hash());
    kept.apply(seat(1), PlayerAction::MulliganKeep).unwrap();
    assert_ne!(asked.snapshot_hash(), kept.snapshot_hash());
}

/// A seat's new hand is its own: the same whether the seat beside it took
/// its mulligan before or after (per-seat streams, `GameRng::for_seat`).
#[test]
fn a_seats_new_hand_does_not_depend_on_who_answered_first() {
    let mut first = open_table(2, 257);
    let mut second = open_table(2, 257);
    let dealt = hand(&first, 1);
    for p in [0, 1] {
        first.apply(seat(p), PlayerAction::MulliganTake).unwrap();
    }
    for p in [1, 0] {
        second.apply(seat(p), PlayerAction::MulliganTake).unwrap();
    }
    assert_ne!(hand(&first, 1), dealt, "the mulligan dealt the same hand");
    for p in [0, 1] {
        assert_eq!(hand(&first, p), hand(&second, p), "seat {p}'s hand");
        assert_eq!(
            library(&first, p),
            library(&second, p),
            "seat {p}'s library"
        );
    }
}

/// The table's stream leaves the mulligans where it entered them, however
/// many were taken: what is random after turn 1 begins does not depend on
/// how anybody mulliganed.
#[test]
fn the_tables_stream_leaves_the_mulligans_where_it_entered_them() {
    let mut kept = open_table(3, 257);
    for p in 0..3 {
        kept.apply(seat(p), PlayerAction::MulliganKeep).unwrap();
    }
    let mut took = open_table(3, 257);
    for _ in 0..3 {
        took.apply(seat(0), PlayerAction::MulliganTake).unwrap();
    }
    took.apply(seat(2), PlayerAction::MulliganTake).unwrap();
    for p in 0..3 {
        took.apply(seat(p), PlayerAction::MulliganKeep).unwrap();
    }
    bottom(&mut took, 0, 2);
    for engine in [&kept, &took] {
        assert_eq!(turns_started(engine), vec![(1, 0)]);
    }
    assert!(took.state().rng.calls() > 0);
    assert_eq!(kept.state().rng.calls(), took.state().rng.calls());
    assert_eq!(kept.state().rng.word_pos(), took.state().rng.word_pos());
    assert_eq!(hand(&kept, 1), hand(&took, 1), "seat 1 only kept");
    assert_eq!(library(&kept, 1), library(&took, 1), "seat 1 only kept");
}

/// A table that only keeps is dealt what it was dealt before the mulligans
/// were answered at once (taken from the engine that asked in seat order).
#[test]
fn a_table_that_keeps_is_dealt_what_it_was_dealt_in_seat_order() {
    let engine = table(3, 257);
    let hands: Vec<Vec<u32>> = (0..3)
        .map(|p| hand(&engine, p).iter().map(|id| id.slot()).collect())
        .collect();
    assert_eq!(
        hands,
        [
            [39, 15, 37, 43, 25, 58, 26],
            [107, 87, 60, 100, 69, 116, 74],
            [165, 147, 171, 138, 141, 135, 146],
        ]
    );
    let folded: Vec<u64> = (0..3)
        .map(|p| {
            library(&engine, p)
                .iter()
                .fold(0xcbf2_9ce4_8422_2325, |h: u64, id| {
                    (h ^ u64::from(id.slot())).wrapping_mul(0x100_0000_01b3)
                })
        })
        .collect();
    assert_eq!(
        folded,
        [
            0xa040_32a7_7157_306e,
            0x1d49_0560_2e2d_d1d8,
            0xfa22_eb93_0ee0_04c0
        ]
    );
    assert_eq!(engine.state().rng.calls(), 177);
}

/// A seat may take mulligans until its opening hand would be zero cards and
/// no further (CR 103.5). One more left a bottom question asking for more
/// cards than the hand holds, which no answer could satisfy.
#[test]
fn no_mulligan_is_taken_past_a_hand_of_zero() {
    let mut engine = open_table(2, 257);
    // The first is free, so the eighth take owes all seven cards.
    for _ in 0..8 {
        engine.apply(seat(0), PlayerAction::MulliganTake).unwrap();
    }
    assert!(
        engine.apply(seat(0), PlayerAction::MulliganTake).is_err(),
        "a ninth mulligan was taken"
    );
    engine.apply(seat(0), PlayerAction::MulliganKeep).unwrap();
    assert!(matches!(
        engine.pending_for(seat(0)),
        Some(Pending::MulliganBottom { count: 7, .. })
    ));
    bottom(&mut engine, 0, 7);
    assert!(hand(&engine, 0).is_empty());
    assert!(engine.pending_for(seat(0)).is_none());
}

fn seat(p: u8) -> PlayerId {
    PlayerId::new(p)
}

fn hand(engine: &Engine<RegistryLookup>, p: u8) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Hand(seat(p)))
        .clone()
}

fn library(engine: &Engine<RegistryLookup>, p: u8) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat(p)))
        .clone()
}

/// Answers a bottom question with the first `n` cards in hand.
fn bottom(engine: &mut Engine<RegistryLookup>, p: u8, n: usize) {
    let objects = hand(engine, p)[..n].to_vec();
    engine
        .apply(seat(p), PlayerAction::ChooseObjects { objects })
        .unwrap();
}

/// Every `TurnStarted` in the journal, as (turn number, active seat).
fn turns_started(engine: &Engine<RegistryLookup>) -> Vec<(u32, u8)> {
    engine
        .journal()
        .entries()
        .iter()
        .filter_map(|e| match &e.event {
            GameEvent::TurnStarted { number, active } => Some((*number, active.get())),
            _ => None,
        })
        .collect()
}
