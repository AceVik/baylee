//! `cards/instants/mv_5/stonefury.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "577554de-78c3-4fa2-a866-06b80280bc97"
fn stonefury() -> CardIndex {
    card_index("577554de-78c3-4fa2-a866-06b80280bc97")
}

/// Stonefury: "deals damage to target creature equal to the number of lands
/// you control."
///
/// Seven lands, two of them Forests that do not help pay: seven damage, read
/// from the journal, and only lands of the caster count (the opponent's Forest
/// is not among them).
#[test]
fn stonefury_deals_damage_equal_to_the_lands_you_control() {
    let lands = [vec![mountain(); 5], vec![forest(); 2]].concat();
    let mut engine = table(
        929,
        &[stonefury()],
        &lands,
        &[],
        &[earth_elemental(), forest()],
    );
    let big = on_battlefield(&engine, them(), earth_elemental()).expect("their Elemental");
    let journal = engine.journal().entries().len();
    cast_at(&mut engine, stonefury(), big);
    let hits: Vec<_> = damage_events(&engine, journal)
        .into_iter()
        .filter(|(_, _, amount, _)| *amount > 0)
        .collect();
    assert_eq!(hits.len(), 1, "one hit: {hits:?}");
    assert!(matches!(hits[0].1, crate::event::DamageTarget::Object(o) if o == big));
    assert_eq!(hits[0].2, 7, "seven lands of mine, not eight with theirs");
    assert!(in_graveyard(&engine, them(), earth_elemental()).is_some());
}
