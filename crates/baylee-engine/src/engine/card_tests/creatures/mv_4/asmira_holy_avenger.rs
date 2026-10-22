//! `cards/creatures/mv_4/asmira_holy_avenger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{ASMIRA_HOLY_AVENGER, GRIZZLY_BEARS, LIGHTNING_BOLT};

#[test]
fn asmira_holy_avenger_gains_counters_at_end_step_for_creatures_that_died() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[LIGHTNING_BOLT])
        .battlefield(0, &[ASMIRA_HOLY_AVENGER, GRIZZLY_BEARS, mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let asmira = on_battlefield(&engine, p0, ASMIRA_HOLY_AVENGER).unwrap();
    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Graveyard);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine
            .state()
            .object(asmira)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "Asmira should get a +1/+1 counter at the end step for the creature that died"
    );
}
