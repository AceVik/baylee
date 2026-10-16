//! `cards/instants/mv_5/narset_s_rebuke.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "247075c5-62f8-41a1-91b3-562ff0aabb30"
fn narset_s_rebuke() -> CardIndex {
    card_index("247075c5-62f8-41a1-91b3-562ff0aabb30")
}

/// Narset's Rebuke: "deals 5 damage to target creature. Add {U}{R}{W}. If that
/// creature would die this turn, exile it instead."
///
/// The Elemental (toughness five) dies to the damage and is exiled instead of
/// dying; blue, red and white are left floating, one each.
#[test]
fn narset_s_rebuke_exiles_what_it_kills_and_adds_three_colours() {
    let mut engine = table(
        928,
        &[narset_s_rebuke()],
        &[mountain(); 5],
        &[],
        &[earth_elemental()],
    );
    let big = on_battlefield(&engine, them(), earth_elemental()).expect("their Elemental");
    cast_at(&mut engine, narset_s_rebuke(), big);

    assert!(
        exiled(&engine, them(), earth_elemental()),
        "exiled instead of dying"
    );
    assert!(in_graveyard(&engine, them(), earth_elemental()).is_none());
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert_eq!(pool.available(ManaColor::White), 1);
    assert_eq!(pool.total(), 3, "and nothing else");
}
