//! `cards/creatures/mv_2/freyalise_supplicant.rs`, played.
//!
//! Oracle:
//! - "{T}, Sacrifice a red or white creature: This creature deals damage to any target equal to half the sacrificed creature's power, rounded down."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn freyalise_supplicant() -> CardIndex {
    card_index("c28a7641-49e3-43b1-a366-8d6e9c8dab9c")
}

/// Activates the Supplicant, answers its target, and returns the creatures
/// the sacrifice then offers: the target is chosen before the cost is paid
/// (CR 602.2b, 601.2c before 601.2h).
#[track_caller]
fn sacrifice_offer(
    engine: &mut Engine<RegistryLookup>,
    objects: &[ObjectId],
    players: &[PlayerId],
) -> Vec<ObjectId> {
    activate(engine, P0, freyalise_supplicant(), 0);
    lk::aim(engine, objects, players);
    let Pending::ChooseCards {
        options,
        prompt: ChoicePrompt::CostSacrifice,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice is asked after the target, got {:?}",
            engine.pending()
        );
    };
    options
}

#[test]
fn freyalise_supplicant_sacrifices_red_or_white_creature_for_half_power_damage_rounded_down() {
    let mut engine = lk::game(
        SEED,
        &[
            freyalise_supplicant(),
            index::SHIVAN_DRAGON,
            index::GRIZZLY_BEARS,
        ],
        &[],
        &[],
        &[],
    );
    let supplicant = lk::obj(&engine, P0, freyalise_supplicant());
    let dragon = lk::obj(&engine, P0, index::SHIVAN_DRAGON);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    let offer = sacrifice_offer(&mut engine, &[], &[P1]);
    assert!(offer.contains(&dragon), "Shivan Dragon is red");
    assert!(
        !offer.contains(&bears),
        "Grizzly Bears is green: not a red or white creature"
    );
    assert!(
        !offer.contains(&supplicant),
        "nor is the green Supplicant itself"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![dragon],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, dragon), Zone::Graveyard);
    assert!(lk::tapped(&engine, supplicant), "{{T}} is part of the cost");
    assert_eq!(
        lk::life(&engine, P1),
        18,
        "half of 5 power rounded down deals 2 damage"
    );
}

/// "A red **or white** creature", and "any target" a creature as well as a
/// player: Savannah Lions' 2 power is 1 damage to the opponent's Bears.
#[test]
fn freyalise_supplicant_takes_a_white_creature_and_hits_a_creature() {
    let mut engine = lk::game(
        SEED,
        &[freyalise_supplicant(), index::SAVANNAH_LIONS],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    let offer = sacrifice_offer(&mut engine, &[their_bears], &[]);
    assert!(offer.contains(&lions), "Savannah Lions is white");
    assert!(
        !offer.contains(&their_bears),
        "the opponent's green Bears is neither P0's nor red or white"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![lions],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, lions), Zone::Graveyard);
    assert_eq!(
        lk::damage_on(&engine, their_bears),
        1,
        "half of 2 power is 1 damage, on the targeted creature"
    );
    assert_eq!(lk::life(&engine, P1), 20, "and none to its controller");
}
