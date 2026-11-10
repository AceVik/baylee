//! Oracle scenarios for Pardic Swordsmith.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn pardic_swordsmith() -> CardIndex {
    card_index("946d770f-0a59-415b-b334-8f7353b96046")
}

#[test]
fn pardic_swordsmith_discards_at_random_for_plus_two_power() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[index::MOUNTAIN; 4])
        .hand(0, &[pardic_swordsmith(), index::FOREST])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, pardic_swordsmith());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, pardic_swordsmith()).unwrap();
    assert_eq!(pt(&e, id), (1, 1));

    activate(&mut e, p0, pardic_swordsmith(), 0);
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, index::FOREST).is_some());
    assert_eq!(pt(&e, id), (3, 1));
}
