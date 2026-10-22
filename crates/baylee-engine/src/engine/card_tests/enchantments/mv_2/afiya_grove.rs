//! `cards/enchantments/mv_2/afiya_grove.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{AFIYA_GROVE, GRIZZLY_BEARS};

#[test]
fn afiya_grove_moves_counters_to_creatures_and_sacrifices_when_empty() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[AFIYA_GROVE])
        .battlefield(0, &[GRIZZLY_BEARS, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, AFIYA_GROVE);
    pass_until(&mut engine, stack_is_empty);

    let grove = on_battlefield(&engine, p0, AFIYA_GROVE).unwrap();
    assert_eq!(
        engine
            .state()
            .object(grove)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        3
    );

    for expected_bear_counters in 1..=3 {
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        aim_at(&mut engine, p0, bear);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine
                .state()
                .object(bear)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            expected_bear_counters
        );
        reach_main_phase(&mut engine, p0);
    }

    assert_eq!(
        engine.state().object(grove).unwrap().zone,
        Zone::Graveyard,
        "Afiya Grove should be sacrificed after its third +1/+1 counter is moved"
    );
}
