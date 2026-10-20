//! `cards/enchantments/mv_6/earthlink.rs`, played.
//!
//! Oracle:
//! - "At the beginning of your upkeep, sacrifice this enchantment unless you pay {2}."
//! - "Whenever a creature dies, that creature's controller sacrifices a land of their choice."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "c6e4a5c6-3c9b-49d5-8288-1be46be657dc"
fn earthlink() -> CardIndex {
    card_index("c6e4a5c6-3c9b-49d5-8288-1be46be657dc")
}

#[test]
fn earthlink_forces_dying_creatures_controller_to_sacrifice_a_land() {
    let mut engine = lk::game(
        SEED,
        &[index::FOREST],
        &[index::GRIZZLY_BEARS, index::MOUNTAIN],
        &[earthlink(), index::LIGHTNING_BOLT],
        &[],
    );
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let their_mountain = lk::obj(&engine, P1, index::MOUNTAIN);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 3),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, earthlink());
    pass_until(&mut engine, stack_is_empty);

    // Kill P1's Grizzly Bears with Lightning Bolt: Earthlink triggers and P1 sacrifices their Mountain.
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[their_bears], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                P1,
                PlayerAction::ChooseObjects {
                    objects: vec![their_mountain],
                },
            )
            .expect("P1 sacrifices their Mountain");
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P1, index::MOUNTAIN).is_none(),
        "that creature's controller (P1) sacrificed their land when their creature died"
    );
    assert!(
        on_battlefield(&engine, P0, index::FOREST).is_some(),
        "P0's land is untouched"
    );
}
