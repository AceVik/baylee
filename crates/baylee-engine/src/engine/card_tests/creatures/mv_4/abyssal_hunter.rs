//! `cards/creatures/mv_4/abyssal_hunter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{ABYSSAL_HUNTER, GRIZZLY_BEARS};

#[test]
fn abyssal_hunter_taps_and_deals_power_damage_to_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[ABYSSAL_HUNTER, swamp()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, ABYSSAL_HUNTER, 0);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    let bear_obj = engine.state().object(bear).unwrap();
    assert!(bear_obj.status.contains(Status::TAPPED));
    assert_eq!(bear_obj.damage, 1);
}
