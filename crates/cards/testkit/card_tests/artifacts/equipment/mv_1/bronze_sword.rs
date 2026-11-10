//! `cards/artifacts/equipment/mv_1/bronze_sword.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn bronze_sword() -> CardIndex {
    card_index("14691783-1246-45ed-a439-c86d7b5eef62")
}

/// Bronze Sword — "Equipped creature gets +2/+0." and "Equip {3}". The price
/// is paid in full (two mana offer nothing), the bonus lands on the equipped
/// Elf alone, and a second Equip moves it.
#[test]
fn bronze_sword_gives_plus_two_power_for_equip_three() {
    plain_pump(bronze_sword(), 3, 2, 0);
}
