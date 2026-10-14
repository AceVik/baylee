//! `cards/artifacts/equipment/mv_1/greataxe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn greataxe() -> CardIndex {
    card_index("843f3373-b8ef-416c-9163-aa1aa6b4a51d")
}

/// Greataxe — "Equipped creature gets +4/+0." and "Equip {5}". The price is
/// paid in full (four mana offer nothing), the bonus lands on the equipped
/// Elf alone, and a second Equip moves it.
#[test]
fn greataxe_gives_plus_four_power_for_equip_five() {
    plain_pump(greataxe(), 5, 4, 0);
}
