//! `cards/creatures/mv_5/lichenthrope.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn lichenthrope() -> CardIndex {
    card_index("ee7f698d-55c5-4a7a-803d-304febe6a758")
}

/// `Lichenthrope` ({3}{G}{G}, 5/5 Creature — Plant Fungus, `Coverage::Partial`)
/// removes a -1/-1 counter from itself at the beginning of your upkeep.
#[test]
fn lichenthrope_removes_m1m1_counter_at_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[lichenthrope()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, lichenthrope());
    pass_until(&mut engine, stack_is_empty);
    let plant = on_battlefield(&engine, p0, lichenthrope()).unwrap();

    {
        let st = engine.dev_state_mut(p0).unwrap();
        st.object_mut(plant)
            .unwrap()
            .counters
            .set(CounterKind::M1M1, 2);
        st.invalidate_projections();
    }
    assert_eq!(counters_on(&engine, plant, CounterKind::M1M1), 2);

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(counters_on(&engine, plant, CounterKind::M1M1), 1);
    assert_eq!(pt(&engine, plant), (4, 4));
}
