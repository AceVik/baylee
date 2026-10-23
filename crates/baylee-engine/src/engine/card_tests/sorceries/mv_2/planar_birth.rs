#[allow(clippy::wildcard_imports)]
use super::*;

fn planar_birth() -> CardIndex {
    card_index("7a9d13b3-3823-463a-acd9-8b7a9d5e121f")
}

#[test]
fn planar_birth_returns_basic_lands_from_graveyards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[planar_birth()])
        .battlefield(0, &[plains(), plains(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dead_forest = on_battlefield(&engine, p0, forest()).expect("forest on bf");
    bury(&mut engine, &[dead_forest]);
    engine.refresh_offer();
    assert!(in_graveyard(&engine, p0, forest()).is_some());

    cast_from_hand(&mut engine, p0, planar_birth());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, forest()).is_some());
}
