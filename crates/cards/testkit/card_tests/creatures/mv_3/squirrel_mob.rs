//! Oracle scenarios for Squirrel Mob.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn squirrel_mob_gets_plus_1_plus_1_for_each_other_squirrel_on_battlefield() {
    let p0 = PlayerId::new(0);
    let card = card_index("451486df-6160-4ff7-b47c-dd3760417a31");

    let mut alone = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[card])
        .start();
    keep_mulligans(&mut alone);
    reach_main_phase(&mut alone, p0);
    let mob_alone = on_battlefield(&alone, p0, card).unwrap();
    assert_eq!(pt(&alone, mob_alone), (2, 2));

    let mut pair = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[card])
        .battlefield(1, &[card])
        .start();
    keep_mulligans(&mut pair);
    reach_main_phase(&mut pair, p0);
    let mob = on_battlefield(&pair, p0, card).unwrap();
    assert_eq!(pt(&pair, mob), (3, 3));
}
