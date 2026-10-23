#[allow(clippy::wildcard_imports)]
use super::*;

fn fledgling_osprey() -> CardIndex {
    card_index("d5482ccf-7c3f-45b8-aaa8-8f6686fe584d")
}

#[test]
fn fledgling_osprey_gains_flying_while_enchanted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[rancor()])
        .battlefield(0, &[fledgling_osprey(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let osprey = on_battlefield(&engine, p0, fledgling_osprey()).unwrap();
    assert!(!keywords(&engine, osprey).contains(KeywordSet::FLYING));

    cast_from_hand(&mut engine, p0, rancor());
    aim_at(&mut engine, p0, osprey);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, osprey).contains(KeywordSet::FLYING));
}
