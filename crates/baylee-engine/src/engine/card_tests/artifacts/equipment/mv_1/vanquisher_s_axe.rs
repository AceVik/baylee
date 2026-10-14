//! `cards/artifacts/equipment/mv_1/vanquisher_s_axe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn vanquisher_s_axe() -> CardIndex {
    card_index("fbdd3ba9-d799-476b-87bc-a689d8e3cff9")
}

/// Vanquisher's Axe — "Equipped creature gets +2/+0." and "Equip {2}". The
/// bonus lands on the equipped Elf alone, and a second Equip moves it.
#[test]
fn vanquisher_s_axe_gives_plus_two_power_for_equip_two() {
    plain_pump(vanquisher_s_axe(), 2, 2, 0);
}
