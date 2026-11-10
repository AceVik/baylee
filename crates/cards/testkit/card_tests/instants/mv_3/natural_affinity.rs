//! `cards/instants/mv_3/natural_affinity.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "09a1ab1b-d9f0-4a2a-a448-3190f85006e4"
fn natural_affinity() -> CardIndex {
    card_index("09a1ab1b-d9f0-4a2a-a448-3190f85006e4")
}

/// `Natural Affinity` (`Coverage::Generated`):
/// "All lands become 2/2 creatures until end of turn. They're still lands."
///
/// Verifies that `Natural Affinity` turns all lands controlled by any player
/// into 2/2 Land Creatures until end of turn, and that they revert to
/// non-creature lands at cleanup.
#[test]
fn natural_affinity_animates_all_lands_as_two_two_creatures_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9505, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .battlefield(1, &[island()])
        .hand(0, &[natural_affinity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_forest = on_battlefield(&engine, p0, forest()).expect("P0 Forest");
    let their_island = on_battlefield(&engine, p1, island()).expect("P1 Island");

    assert!(!types(&engine, my_forest).contains(TypeSet::CREATURE));

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, natural_affinity());
    pass_until(&mut engine, stack_is_empty);

    for land in [my_forest, their_island] {
        let kinds = types(&engine, land);
        assert!(
            kinds.contains(TypeSet::LAND) && kinds.contains(TypeSet::CREATURE),
            "animated land must be both Land and Creature, got {kinds:?}"
        );
        assert_eq!(pt(&engine, land), (2, 2), "animated land is a 2/2");
    }

    reach_their_main_phase(&mut engine, p1);
    for land in [my_forest, their_island] {
        let kinds = types(&engine, land);
        assert!(
            kinds.contains(TypeSet::LAND) && !kinds.contains(TypeSet::CREATURE),
            "land reverts to non-creature Land after cleanup, got {kinds:?}"
        );
    }
}
