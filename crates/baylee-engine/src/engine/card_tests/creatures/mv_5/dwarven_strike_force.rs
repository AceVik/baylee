//! Oracle scenarios for Dwarven Strike Force.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn dwarven_strike_force() -> CardIndex {
    card_index("66dd0e68-f5ec-45e5-991f-d588aa726387")
}

#[test]
fn dwarven_strike_force_discards_at_random_for_first_strike_and_haste() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[index::MOUNTAIN; 5])
        .hand(0, &[dwarven_strike_force(), index::FOREST])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, dwarven_strike_force());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, dwarven_strike_force()).unwrap();
    assert!(!keywords(&e, id).contains(KeywordSet::FIRST_STRIKE));
    assert!(!keywords(&e, id).contains(KeywordSet::HASTE));

    activate(&mut e, p0, dwarven_strike_force(), 0);
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, index::FOREST).is_some());
    assert!(keywords(&e, id).contains(KeywordSet::FIRST_STRIKE));
    assert!(keywords(&e, id).contains(KeywordSet::HASTE));
}
