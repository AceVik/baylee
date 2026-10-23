#[allow(clippy::wildcard_imports)]
use super::*;

fn despondency() -> CardIndex {
    card_index("a68ac1ba-f51e-446c-abf0-7a2bd4d65a8c")
}

#[test]
fn despondency_weakens_creature_and_returns_to_hand_when_put_into_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[despondency()])
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, despondency());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, bear);
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (0, 2));

    let aura = on_battlefield(&engine, p0, despondency()).expect("attached");
    kill(&mut engine, aura);

    assert!(in_hand(&engine, p0, despondency()).is_some());
}
