//! Fixed, bounded custom-mode supplies. Authority comes from the run service.

use serde::{Deserialize, Serialize};

/// Original Baylee objects resolved through the engine's registry seam.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum TowerSupply {
    /// One charge: gain five life through the ordinary stack.
    HealingPotion,
    /// One charge: add one mana of any color through a mana ability.
    ManaPotion,
    /// An ordinary green 1/1 Plant on the starting battlefield.
    Seedling,
}

/// Supplies a custom-mode seat brings into one frozen duel.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct SeatSupplies {
    /// Zero-based seat.
    pub seat: u8,
    /// Healing charges (combined potion capacity is two).
    pub healing: u8,
    /// Mana charges (combined potion capacity is two).
    pub mana: u8,
    /// Whether the seat starts with its one seedling.
    pub seedling: bool,
}

pub(super) fn valid_list(rules: &[SeatSupplies], seats: usize, custom: bool) -> bool {
    rules.is_empty()
        || (custom
            && rules.len() <= seats
            && rules.iter().enumerate().all(|(i, rule)| {
                usize::from(rule.seat) < seats
                    && u16::from(rule.healing) + u16::from(rule.mana) <= 2
                    && !rules[..i].iter().any(|previous| previous.seat == rule.seat)
            }))
}
