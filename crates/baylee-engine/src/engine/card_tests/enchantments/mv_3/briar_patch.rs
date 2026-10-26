//! `cards/enchantments/mv_3/briar_patch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "247793af-be9e-47c7-8887-e99e49478b0b"
fn briar_patch() -> CardIndex {
    card_index("247793af-be9e-47c7-8887-e99e49478b0b")
}

/// Briar Patch ({1}{G}{G} Enchantment):
/// "Whenever a creature attacks you, it gets -1/-0 until end of turn."
#[test]
fn briar_patch_shrinks_creatures_attacking_its_controller_until_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[briar_patch(), index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[],
        &[],
    );
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let their_giant = lk::obj(&engine, P1, index::HILL_GIANT);

    // P0's own creature attacking P1 is not attacking P0, so Briar Patch does not shrink it.
    lk::attack(&mut engine, &[my_bears]);
    assert_eq!(
        pt(&engine, my_bears),
        (2, 2),
        "Briar Patch does not shrink creatures attacking an opponent"
    );
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P1), 18);

    // On P1's turn, Hill Giant attacks P0 and gets -1/-0 until end of turn.
    reach_their_main_phase(&mut engine, P1);
    lk::attack(&mut engine, &[their_giant]);
    assert_eq!(
        pt(&engine, their_giant),
        (2, 3),
        "Briar Patch gave attacking Hill Giant -1/-0 until end of turn"
    );
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(
        lk::life(&engine, P0),
        18,
        "shrunk Hill Giant dealt 2 combat damage instead of 3"
    );
}
