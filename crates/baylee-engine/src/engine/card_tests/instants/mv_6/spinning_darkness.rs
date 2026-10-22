//! `cards/instants/mv_6/spinning_darkness.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{HILL_GIANT, SPINNING_DARKNESS};

/// `Spinning Darkness` ({4}{B}{B}, Instant, Partial) deals 3 damage to target
/// nonblack creature and you gain 3 life.
#[test]
fn spinning_darkness_deals_three_damage_to_nonblack_creature_and_gains_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[SPINNING_DARKNESS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, p0, SPINNING_DARKNESS);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, HILL_GIANT).is_some());
    assert_eq!(engine.state().players[0].life, 23);
}
