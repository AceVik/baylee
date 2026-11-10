//! `cards/artifacts/equipment/mv_1/trusty_machete.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn trusty_machete() -> CardIndex {
    card_index("394fae8f-3757-4e08-b97c-5d7c451f4e72")
}

/// Trusty Machete — "Equipped creature gets +2/+1." and "Equip {2}". The
/// bonus lands on the equipped Elf alone, and a second Equip moves it.
#[test]
fn trusty_machete_gives_plus_two_plus_one_for_equip_two() {
    plain_pump(trusty_machete(), 2, 2, 1);
}
