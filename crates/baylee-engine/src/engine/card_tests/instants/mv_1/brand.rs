#[allow(clippy::wildcard_imports)]
use super::*;

fn brand() -> CardIndex {
    card_index("eec7500b-418d-4387-beb2-d53e047c1d22")
}

#[test]
fn brand_regains_control_of_permanents_you_own() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[brand()])
        .battlefield(0, &[mountain(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    engine.state.object_mut(bears).unwrap().controller = p1;
    engine.state.invalidate_projections();

    cast_from_hand(&mut engine, p0, brand());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bears).unwrap().controller, p0);
}
