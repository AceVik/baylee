#[allow(clippy::wildcard_imports)]
use super::*;

fn damping_engine() -> CardIndex {
    card_index("3584a055-5220-4721-89f8-de33f779122f")
}

#[test]
fn damping_engine_casts_and_enters_battlefield_as_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[damping_engine()])
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, damping_engine());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, damping_engine()).is_some());
}
