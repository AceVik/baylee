//! `cards/instants/mv_1/burnt_offering.rs`, played.
//!
//! Oracle:
//! - "As an additional cost to cast this spell, sacrifice a creature."
//! - "Add X mana in any combination of {B} and/or {R}, where X is the sacrificed creature's mana value."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "86eb30a0-0beb-42db-9ddf-cf8be6c99dd3"
fn burnt_offering() -> CardIndex {
    card_index("86eb30a0-0beb-42db-9ddf-cf8be6c99dd3")
}

#[test]
fn burnt_offering_sacrifices_creature_and_adds_mana_in_any_combination_of_b_and_r() {
    let mut engine = lk::game(
        SEED,
        &[index::SWAMP, index::HILL_GIANT],
        &[],
        &[burnt_offering()],
        &[],
    );
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);

    cast_from_hand(&mut engine, P0, burnt_offering());
    // Additional cost: sacrifice Hill Giant (the only creature P0 controls; if prompted, choose it).
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![giant],
                },
            )
            .expect("sacrifice Hill Giant");
    }
    assert!(
        on_battlefield(&engine, P0, index::HILL_GIANT).is_none(),
        "Hill Giant is sacrificed as an additional cost to cast Burnt Offering"
    );

    // Resolve Burnt Offering: Hill Giant has mana value 4, so choose 2 {B} and 2 {R}.
    for pick in [
        ManaColor::Black,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Red,
    ] {
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseColor { .. })
        });
        engine
            .apply(P0, PlayerAction::ChooseColor(pick))
            .expect("choose mana color");
    }
    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 2);
    assert_eq!(pool.available(ManaColor::Red), 2);
}
