#[allow(clippy::wildcard_imports)]
use super::*;

fn exhume() -> CardIndex {
    card_index("fbe61f74-1b3c-4e12-8758-7029872c9ff1")
}

#[test]
fn exhume_reanimates_creature_from_each_players_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[exhume()])
        .battlefield(0, &[swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[wild_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");
    let elephant = on_battlefield(&engine, p1, wild_elephant()).expect("seated");
    bury(&mut engine, &[elves, elephant]);

    cast_from_hand(&mut engine, p0, exhume());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p1, wild_elephant()).is_some());
}
