#[allow(clippy::wildcard_imports)]
use super::*;

fn fire_ants() -> CardIndex {
    card_index("c23543b7-f637-4a7c-9b72-ec411cd5488e")
}

#[test]
fn fire_ants_damages_each_other_creature_without_flying() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[fire_ants(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), ornithopter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ants = on_battlefield(&engine, p0, fire_ants()).expect("seated");
    activate(&mut engine, p0, fire_ants(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert_eq!(engine.state().object(ants).unwrap().damage, 0);
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    let flyer = on_battlefield(&engine, p1, ornithopter()).unwrap();
    assert_eq!(engine.state().object(flyer).unwrap().damage, 0);
}
