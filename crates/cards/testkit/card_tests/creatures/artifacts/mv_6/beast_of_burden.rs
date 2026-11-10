//! `cards/creatures/artifacts/mv_6/beast_of_burden.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn beast_of_burden() -> CardIndex {
    card_index("67389ccc-dacc-4ce3-a9a2-f551a0d6e9d3")
}

/// Beast of Burden — "power and toughness are each equal to the number of
/// creatures on the battlefield": every creature of either seat, itself
/// included, and no artifact that is not a creature.
#[test]
fn beast_of_burden_counts_every_creature_on_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[beast_of_burden(), gray_ogre(), sol_ring()])
        .battlefield(1, &[wild_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let beast = on_battlefield(&engine, p0, beast_of_burden()).expect("seated");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("seated");
    assert_eq!(
        pt(&engine, beast),
        (3, 3),
        "Beast, Ogre, Elephant; the Ring is no creature"
    );
    kill(&mut engine, ogre);
    assert_eq!(pt(&engine, beast), (2, 2), "one fewer creature");
}
