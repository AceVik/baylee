#[allow(clippy::wildcard_imports)]
use super::*;

fn diabolic_servitude() -> CardIndex {
    card_index("e275cbfb-3d25-4189-84f0-3162eadc7d65")
}

#[test]
fn diabolic_servitude_reanimates_creature_from_graveyard_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[diabolic_servitude()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    bury(&mut engine, &[bear]);
    let bear_in_gy = in_graveyard(&engine, p0, grizzly_bears()).expect("in graveyard");

    cast_from_hand(&mut engine, p0, diabolic_servitude());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, bear_in_gy);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
}
