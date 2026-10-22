//! `cards/creatures/mv_4/cinder_giant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CINDER_GIANT, GRIZZLY_BEARS, HILL_GIANT};

/// `Cinder Giant` ({3}{R}, Creature — Giant 5/3) deals 2 damage to each other
/// creature you control at the beginning of your upkeep.
#[test]
fn cinder_giant_deals_2_damage_to_each_other_creature_you_control_on_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                GRIZZLY_BEARS,
                HILL_GIANT,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[CINDER_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, CINDER_GIANT);
    pass_until(&mut engine, stack_is_empty);

    let cinder = on_battlefield(&engine, p0, CINDER_GIANT).unwrap();
    let hill_giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == p0
            && e.state().turn.phase == Phase::FirstMain
    });

    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().object(hill_giant).unwrap().damage, 2);
    assert_eq!(engine.state().object(cinder).unwrap().damage, 0);
}
