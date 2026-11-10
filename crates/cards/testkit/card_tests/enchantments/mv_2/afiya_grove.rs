//! `cards/enchantments/mv_2/afiya_grove.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn afiya_grove() -> CardIndex {
    card_index("056a98aa-f3f7-4ac9-9755-3e8a60a59abb")
}

#[test]
fn afiya_grove_moves_counters_to_creatures_and_sacrifices_when_empty() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[afiya_grove()])
        .battlefield(0, &[GRIZZLY_BEARS, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, afiya_grove());
    pass_until(&mut engine, stack_is_empty);

    let grove = on_battlefield(&engine, p0, afiya_grove()).unwrap();
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
        // While counters are left on it the Grove stays; the state trigger
        // waits for none.
        if expected_bear_counters < 3 {
            let grove_object = engine.state().object(grove).unwrap();
            assert_eq!(grove_object.zone, Zone::Battlefield);
            assert_eq!(
                grove_object.counters.get(CounterKind::P1P1),
                3 - expected_bear_counters
            );
        }
        reach_main_phase(&mut engine, p0);
    }

    assert_eq!(
        engine.state().object(grove).unwrap().zone,
        Zone::Graveyard,
        "Afiya Grove should be sacrificed after its third +1/+1 counter is moved"
    );
}
