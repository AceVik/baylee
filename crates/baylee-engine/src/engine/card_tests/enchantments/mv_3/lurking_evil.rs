#[allow(clippy::wildcard_imports)]
use super::*;

fn lurking_evil() -> CardIndex {
    card_index("81d6b42e-9c41-4d1e-bd28-4662e1fde572")
}

#[test]
fn lurking_evil_pays_life_to_become_4_4_horror_with_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[lurking_evil()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let perm = on_battlefield(&engine, p0, lurking_evil()).expect("seated");
    activate(&mut engine, p0, lurking_evil(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, perm), (4, 4));
    assert_eq!(engine.state().players[0].life, 19);
}
