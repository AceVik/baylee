//! `cards/enchantments/mv_4/dwarven_armory.rs`, played.
//!
//! Oracle:
//! - "{2}, Sacrifice a land: Put a +2/+2 counter on target creature. Activate only during any upkeep step."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "aba53a54-8666-430d-899e-45ad9f783a4a"
fn dwarven_armory() -> CardIndex {
    card_index("aba53a54-8666-430d-899e-45ad9f783a4a")
}

#[test]
fn dwarven_armory_activates_only_during_any_upkeep_and_puts_plus_two_plus_two_counter() {
    let mut engine = lk::game(
        SEED,
        &[dwarven_armory(), index::MOUNTAIN, index::GRIZZLY_BEARS],
        &[],
        &[],
        &[],
    );
    let mountain = lk::obj(&engine, P0, index::MOUNTAIN);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);
    assert!(
        !lk::offered(&engine, dwarven_armory(), P0, 0),
        "cannot activate Dwarven Armory during a main phase"
    );

    // Advance to P1's upkeep step ("any upkeep step"): now Dwarven Armory is activatable.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);
    activate(&mut engine, P0, dwarven_armory(), 0);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        lk::aim(&mut engine, &[bears], &[]);
    }
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![mountain],
                },
            )
            .expect("sacrifice Mountain");
    }
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        lk::aim(&mut engine, &[bears], &[]);
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine.state().object(mountain).unwrap().zone == Zone::Graveyard,
        "land was sacrificed as part of the activation cost"
    );
    assert_eq!(
        pt(&engine, bears),
        (4, 4),
        "Grizzly Bears has a +2/+2 counter on it"
    );
}
