//! `cards/artifacts/equipment/mv_1/honed_khopesh.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn honed_khopesh() -> CardIndex {
    card_index("f2142c41-ff66-4276-b426-fd945ed9b0f6")
}

/// Honed Khopesh — "Equipped creature gets +1/+1." and "Equip {1}". The
/// bonus lands on the equipped Elf alone, and a second Equip moves it.
#[test]
fn honed_khopesh_gives_plus_one_plus_one_for_equip_one() {
    plain_pump(honed_khopesh(), 1, 1, 1);
}
