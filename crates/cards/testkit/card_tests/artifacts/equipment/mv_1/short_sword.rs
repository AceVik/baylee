//! `cards/artifacts/equipment/mv_1/short_sword.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn short_sword() -> CardIndex {
    card_index("e842a4e5-ea0b-43ae-9ed8-93e30f4e7b8a")
}

/// Short Sword — "Equipped creature gets +1/+1." and "Equip {1}". The bonus
/// lands on the equipped Elf alone, and a second Equip moves it.
#[test]
fn short_sword_gives_plus_one_plus_one_for_equip_one() {
    plain_pump(short_sword(), 1, 1, 1);
}
