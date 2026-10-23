#[allow(clippy::wildcard_imports)]
use super::*;

fn lifeline() -> CardIndex {
    card_index("f475b636-eb7f-4dd7-bc87-a1af07064ffe")
}

#[test]
fn lifeline_returns_dying_creature_when_another_creature_is_on_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lifeline(), wild_elephant(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");
    kill(&mut engine, elves);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
}
