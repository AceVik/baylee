#[allow(clippy::wildcard_imports)]
use super::*;

fn no_rest_for_the_wicked() -> CardIndex {
    card_index("d03fd358-ff3f-45ab-9601-07dbb535cb51")
}

#[test]
fn no_rest_for_the_wicked_returns_creatures_that_died_this_turn_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[no_rest_for_the_wicked(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    bury(&mut engine, &[bear]);
    engine.refresh_offer();

    activate(&mut engine, p0, no_rest_for_the_wicked(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
}
