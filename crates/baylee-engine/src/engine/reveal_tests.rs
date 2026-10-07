//! A card an effect reveals is journalled as `GameEvent::Revealed`, the one
//! door through which a reveal reaches every player (CR 701.20a): the game
//! log names what it shows to every seat, and nothing else does.
//!
//! The searches derive their reveal from the filter and the destination
//! (`search_tests`). These are the two other sentences in the pool that
//! reveal a chosen card and journalled nothing: Vendilion Clique's "that
//! player reveals the chosen card" and Karn, the Great Creator's "you may
//! reveal an artifact card you own from outside the game".

use super::testkit::{
    Duel, RegistryLookup, card_index, cast_from_hand, keep_mulligans, on_battlefield, pass_until,
    reach_main_phase, stack_is_empty, walk_to_own_main,
};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::{CardIndex, ObjectId};

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn counterspell() -> CardIndex {
    card_index("cc187110-1148-4090-bbb8-e205694a39f5")
}
fn vendilion_clique() -> CardIndex {
    card_index("244d4807-0802-41bc-9460-55ac38a28a72")
}
fn karn_the_great_creator() -> CardIndex {
    card_index("a20dd48d-d344-4db1-b0e9-a2b71c3cc9d1")
}
fn chromatic_lantern() -> CardIndex {
    card_index("539f5396-d99a-417d-a84c-dff7930b5900")
}

/// Every `Revealed` event in the journal: who revealed, and what.
fn revealed(engine: &Engine<RegistryLookup>) -> Vec<(PlayerId, Vec<ObjectId>)> {
    engine
        .journal()
        .entries()
        .iter()
        .filter_map(|e| match &e.event {
            crate::event::GameEvent::Revealed { player, cards } => Some((*player, cards.clone())),
            _ => None,
        })
        .collect()
}

/// Vendilion Clique pointed at the opponent: the card its controller picks
/// out of that hand is revealed by its owner, and nothing else in the hand
/// is.
#[test]
fn the_card_vendilion_clique_chooses_is_revealed_by_its_owner() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(29, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[vendilion_clique()])
        .hand(1, &[counterspell(), counterspell()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    cast_from_hand(&mut engine, p0, vendilion_clique());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    let chosen = options[0];
    let before = revealed(&engine).len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        revealed(&engine)[before..],
        [(p1, vec![chosen])],
        "the chosen card, and only it, is shown to the table by its owner"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p1)).first(),
        Some(&chosen),
        "and it is on the bottom of their library"
    );
}

/// Karn's −2 from outside the game: the wished-for card is revealed on its
/// way into the hand.
#[test]
fn karns_wish_reveals_the_card_it_takes_from_outside_the_game() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[karn_the_great_creator()])
        .sideboard(0, &[chromatic_lantern()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let karn = on_battlefield(&engine, p0, karn_the_great_creator()).expect("Karn");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: karn,
                ability_index: 2,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    let wished = options[0];
    let before = revealed(&engine).len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wished],
            },
        )
        .unwrap();

    assert_eq!(revealed(&engine)[before..], [(p0, vec![wished])]);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&wished),
        "the card is in the hand"
    );
}
