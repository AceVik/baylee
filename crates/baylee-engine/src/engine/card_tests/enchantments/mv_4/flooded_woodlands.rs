//! `cards/enchantments/mv_4/flooded_woodlands.rs`, played.
//!
//! Oracle:
//! - "Green creatures can't attack unless their controller sacrifices a land of their choice for each green creature they control that's attacking."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn flooded_woodlands() -> CardIndex {
    card_index("413172aa-b508-4e67-b68e-6a749d2e8130")
}

fn declare(engine: &mut Engine<RegistryLookup>, attackers: &[ObjectId]) {
    let offered = lk::attackers_offered(engine);
    assert!(attackers.iter().all(|id| offered.contains(id)));
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|id| (*id, Defender::Player(P1)))
                    .collect(),
            },
        )
        .expect("the declaration has a legal sacrifice payment");
}

fn sacrifice_options(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt: crate::choice::ChoicePrompt::CostSacrifice,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "a land sacrifice choice was expected, got {:?}",
            engine.pending()
        );
    };
    assert_eq!((player, min, max), (P0, 1, 1));
    options
}

fn sacrifice(engine: &mut Engine<RegistryLookup>, land: ObjectId) {
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("the offered land pays the attack cost");
}

/// The opponent's Woodlands tax P0's green attacker one land of P0's choice;
/// the white attacker costs nothing, and the opponent's land is not offered.
#[test]
fn flooded_woodlands_lets_the_attacking_controller_choose_which_land_to_sacrifice() {
    let mut engine = lk::game(
        SEED,
        &[
            index::GRIZZLY_BEARS,
            index::SAVANNAH_LIONS,
            index::FOREST,
            index::FOREST,
        ],
        &[flooded_woodlands(), index::FOREST],
        &[],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let forests = all_on_battlefield(&engine, P0, index::FOREST);
    let kept = forests[0];
    let chosen = forests[1];
    let opponents = lk::obj(&engine, P1, index::FOREST);

    declare(&mut engine, &[bears, lions]);
    let options = sacrifice_options(&engine);
    assert_eq!(options, forests, "exactly P0's two lands are offered");
    assert!(!options.contains(&opponents));
    assert_eq!(lk::zone_of(&engine, chosen), Zone::Battlefield);
    assert!(!lk::tapped(&engine, bears));
    assert!(!lk::tapped(&engine, lions));
    assert!(!engine.state().combat.is_attacking(bears));

    let before = (
        engine.snapshot_hash(),
        engine.state().journal.len(),
        format!("{:?}", engine.pending()),
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![opponents],
                },
            )
            .is_err(),
        "the Woodlands' controller cannot be made to pay P0's cost"
    );
    assert_eq!(
        (
            engine.snapshot_hash(),
            engine.state().journal.len(),
            format!("{:?}", engine.pending()),
        ),
        before,
        "an illegal choice is atomic"
    );

    sacrifice(&mut engine, chosen);
    assert_eq!(lk::zone_of(&engine, chosen), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, kept), Zone::Battlefield);
    assert_eq!(lk::zone_of(&engine, opponents), Zone::Battlefield);
    assert!(lk::tapped(&engine, bears));
    assert!(lk::tapped(&engine, lions));
}

/// With no land to sacrifice, a green creature can't attack; a white one
/// still may.
#[test]
fn flooded_woodlands_keeps_a_green_creature_home_without_a_land() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::SAVANNAH_LIONS],
        &[flooded_woodlands()],
        &[],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);

    let offered = lk::attackers_offered(&mut engine);
    assert!(
        !offered.contains(&bears),
        "no land to sacrifice, no green attacker"
    );
    assert!(offered.contains(&lions), "the white Lions are not taxed");
}

/// Two green attackers cost two different lands, and no land is sacrificed
/// until the second choice has been made.
#[test]
fn flooded_woodlands_charges_a_distinct_chosen_land_for_each_green_attacker() {
    let mut engine = lk::game(
        SEED,
        &[
            flooded_woodlands(),
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
            index::FOREST,
            index::FOREST,
            index::FOREST,
        ],
        &[],
        &[],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let elves = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let forests = all_on_battlefield(&engine, P0, index::FOREST);

    declare(&mut engine, &[bears, elves]);
    assert_eq!(sacrifice_options(&engine), forests);
    sacrifice(&mut engine, forests[0]);
    let second = sacrifice_options(&engine);
    assert_eq!(
        second,
        forests[1..],
        "the first choice cannot be paid twice"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        0,
        "nothing is paid between the choices"
    );
    assert!(!lk::tapped(&engine, bears) && !lk::tapped(&engine, elves));

    sacrifice(&mut engine, forests[2]);
    assert_eq!(lk::zone_of(&engine, forests[0]), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, forests[1]), Zone::Battlefield);
    assert_eq!(lk::zone_of(&engine, forests[2]), Zone::Graveyard);
    assert!(engine.state().combat.is_attacking(bears));
    assert!(engine.state().combat.is_attacking(elves));
}
