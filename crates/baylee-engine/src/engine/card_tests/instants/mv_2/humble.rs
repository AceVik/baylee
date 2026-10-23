#[allow(clippy::wildcard_imports)]
use super::*;

fn humble() -> CardIndex {
    card_index("d2d41f12-18d8-473b-9308-3f02d7d791f0")
}

#[test]
fn humble_removes_abilities_and_sets_target_creature_to_0_1() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[humble()])
        .battlefield(0, &[plains(), plains(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, humble());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, bear);
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (0, 1));
}
