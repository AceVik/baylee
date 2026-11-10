//! Oracle scenarios for Frenetic Ogre.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn frenetic_ogre() -> CardIndex {
    card_index("6eb4b649-c578-4b29-8da1-7c757412bec0")
}

#[test]
fn frenetic_ogre_discards_at_random_for_plus_three_power() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[index::MOUNTAIN; 6])
        .hand(0, &[frenetic_ogre(), index::FOREST])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, frenetic_ogre());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, frenetic_ogre()).unwrap();
    assert_eq!(pt(&e, id), (2, 3));

    activate(&mut e, p0, frenetic_ogre(), 0);
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, index::FOREST).is_some());
    assert_eq!(pt(&e, id), (5, 3));
}
