//! `cards/creatures/artifacts/mv_1/straw_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Straw Golem` prints `When an opponent casts a creature spell, sacrifice this creature.`
/// under `Coverage::Implemented`. In this test, seat 0 controls the 2/3 artifact creature.
/// When the opponent casts `llanowar_elves()`, the trigger triggers, goes on the stack, and
/// upon resolution forces seat 0 to sacrifice `Straw Golem` to their graveyard.
#[test]
fn straw_golem_sacrifices_itself_when_opponent_casts_a_creature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[straw_golem()])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);
    assert!(on_battlefield(&engine, p0, straw_golem()).is_some());

    cast_from_hand(&mut engine, p1, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, straw_golem()).is_none(),
        "`Straw Golem` should no longer be on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, straw_golem()).is_some(),
        "`Straw Golem` should be sacrificed to the graveyard"
    );
}
