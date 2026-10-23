#[allow(clippy::wildcard_imports)]
use super::*;

fn wirecat() -> CardIndex {
    card_index("57828973-df3d-4288-988d-147a3016120e")
}

fn veil_of_birds() -> CardIndex {
    card_index("fc6278e5-7eb1-4590-880e-fe9efa0ec4d9")
}

#[test]
fn wirecat_gains_cant_attack_and_cant_block_when_enchantment_on_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[veil_of_birds()])
        .battlefield(0, &[wirecat(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cat = on_battlefield(&engine, p0, wirecat()).expect("wirecat on bf");
    assert!(!keywords(&engine, cat).contains(KeywordSet::CANT_ATTACK));

    cast_from_hand(&mut engine, p0, veil_of_birds());
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, cat).contains(KeywordSet::CANT_ATTACK));
    assert!(keywords(&engine, cat).contains(KeywordSet::CANT_BLOCK));
}
