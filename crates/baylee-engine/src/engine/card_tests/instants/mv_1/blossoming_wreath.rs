//! `cards/instants/mv_1/blossoming_wreath.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn blossoming_wreath() -> CardIndex {
    card_index("fbd99a88-8926-455e-8e23-58fed6d14719")
}

/// `Blossoming Wreath` ({G}, Instant) gains life equal to the number of
/// creature cards in your graveyard.
#[test]
fn blossoming_wreath_gains_life_for_each_creature_in_your_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, GRIZZLY_BEARS)
        .battlefield(0, &[forest()])
        .hand(0, &[blossoming_wreath()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 2);

    cast_from_hand(&mut engine, p0, blossoming_wreath());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[p0.get() as usize].life, 22);
}
