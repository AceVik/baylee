//! `cards/instants/mv_1/fatal_blow.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{FATAL_BLOW, HILL_GIANT, SHOCK};

/// `Fatal Blow` ({B}, Instant) destroys target creature that was dealt damage
/// this turn so it can't be regenerated.
#[test]
fn fatal_blow_destroys_creature_that_was_dealt_damage_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[mountain(), swamp()])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[SHOCK, FATAL_BLOW])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, SHOCK);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(giant).unwrap().damage, 2);

    cast_with_floating(&mut engine, p0, FATAL_BLOW);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, HILL_GIANT).is_some());
}
