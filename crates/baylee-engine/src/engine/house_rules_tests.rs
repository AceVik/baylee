//! The three deliberate departures from the Comprehensive Rules.
//!
//! These are *house rules*, not bugs, and that is exactly why they need
//! tests: every one of them looks like a rules mistake to anyone reading the
//! engine against CR, so nothing but a test says "this is on purpose".
//!
//! 1. The first mulligan is free (CR 103.5 charges for every one).
//! 2. With three or more players nobody skips their first draw step
//!    (CR 103.8a skips it for the starting player in *every* game).
//! 3. A real endless loop resolves once and is then broken (CR 104.4b makes
//!    it a draw) — covered in `loop_tests`, since it needs a loop to run.

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

/// A table of `seats` players, all AI-controlled, each on a plain deck.
fn table(seats: usize, seed: u64) -> Engine<RegistryLookup> {
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
                starting_life: None,
                starting_hand: None,
                starting_battlefield: vec![],
                emblems: vec![],
                team: None,
            })
            .collect(),
    };
    let mut engine = Engine::new(&preset, RegistryLookup).expect("table starts");
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

/// House rule 2: with three or more players nobody skips, so the starting
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
