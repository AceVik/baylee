//! `cards/enchantments/auras/mv_3/ancestral_mask.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "db5380ed-ba28-4ea2-abc3-4998e2022903"
fn ancestral_mask() -> CardIndex {
    card_index("db5380ed-ba28-4ea2-abc3-4998e2022903")
}

/// Ancestral Mask ({2}{G} Enchantment — Aura):
/// "Enchant creature"
/// "Enchanted creature gets +2/+2 for each other enchantment on the battlefield."
#[test]
fn ancestral_mask_grants_plus_two_plus_two_for_each_other_enchantment() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::CONCORDANT_CROSSROADS],
        &[index::BLACK_MARKET],
        &[ancestral_mask()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    assert_eq!(pt(&engine, bears), (2, 2));

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
    );
    lk::cast_at(&mut engine, P0, ancestral_mask(), &[bears], &[]);

    // Two other enchantments on the battlefield (Concordant Crossroads + Black Market) -> +4/+4 -> 6/6.
    // Ancestral Mask itself is not counted ("each other enchantment").
    assert_eq!(
        pt(&engine, bears),
        (6, 6),
        "enchanted creature gets +2/+2 for each of the two other enchantments on the battlefield"
    );
    let _ = P1;
}
