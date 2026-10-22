//! `cards/instants/mv_2/alarum.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{ALARUM, GRIZZLY_BEARS};

#[test]
fn alarum_untaps_nonattacking_creature_and_gives_plus_one_plus_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[ALARUM])
        .battlefield(0, &[GRIZZLY_BEARS, plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    engine.dev_state_mut(p0).unwrap().set_tapped(bear, true);

    cast_from_hand(&mut engine, p0, ALARUM);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    let bear_obj = engine.state().object(bear).unwrap();
    assert!(!bear_obj.status.contains(Status::TAPPED));
    assert_eq!(pt(&engine, bear), (3, 5));
}
