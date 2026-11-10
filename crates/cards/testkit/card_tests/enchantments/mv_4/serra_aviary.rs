//! `cards/enchantments/mv_4/serra_aviary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn serra_aviary() -> CardIndex {
    card_index("7a06483b-e71c-4d11-86ea-3b48e2a9cdaf")
}

/// "Creatures with flying get +1/+1": every flyer, whoever controls it, and
/// no ground creature.
#[test]
fn serra_aviary_pumps_every_flyer_and_no_other_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[serra_aviary(), serra_angel(), grizzly_bears()])
        .battlefield(1, &[serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, serra_angel()).expect("seated");
    let theirs = on_battlefield(&engine, p1, serra_angel()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, mine), (5, 5), "my flyer: 4/4 plus 1/1");
    assert_eq!(pt(&engine, theirs), (5, 5), "an opposing flyer too");
    assert_eq!(pt(&engine, bears), (2, 2), "a creature without flying");
}
