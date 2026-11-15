//! Stable presentation lookup for original life relics; no Oracle identity.
#[allow(clippy::wildcard_imports)]
use super::*;

/// Chalice, Hearthstone, Blood Amber, Life Coil; each Common through Mythic Rare.
pub const LIFE_RELICS: [[&TokenDef; 4]; 4] = [
    [
        &TOWER_CHALICE_COMMON,
        &TOWER_CHALICE_UNCOMMON,
        &TOWER_CHALICE_RARE,
        &TOWER_CHALICE_MYTHIC,
    ],
    [
        &TOWER_HEARTH_COMMON,
        &TOWER_HEARTH_UNCOMMON,
        &TOWER_HEARTH_RARE,
        &TOWER_HEARTH_MYTHIC,
    ],
    [
        &TOWER_AMBER_COMMON,
        &TOWER_AMBER_UNCOMMON,
        &TOWER_AMBER_RARE,
        &TOWER_AMBER_MYTHIC,
    ],
    [
        &TOWER_COIL_COMMON,
        &TOWER_COIL_UNCOMMON,
        &TOWER_COIL_RARE,
        &TOWER_COIL_MYTHIC,
    ],
];

const TEXT: [[&str; 4]; 4] = [
    [
        "At the beginning of your upkeep, you gain 1 life.",
        "At the beginning of your upkeep, you gain 2 life.",
        "At the beginning of your upkeep, you gain 3 life.",
        "At the beginning of your upkeep, you gain 5 life.",
    ],
    [
        "Whenever a creature you control enters, you gain 1 life.",
        "Whenever a creature you control enters, you gain 2 life.",
        "Whenever a creature you control enters, you gain 3 life.",
        "Whenever a creature you control enters, you gain 4 life.",
    ],
    [
        "{T}, Remove two charge counters from this artifact: It deals 1 damage to any target.",
        "{T}, Remove two charge counters from this artifact: It deals 2 damage to any target.",
        "{T}, Remove two charge counters from this artifact: It deals 3 damage to any target.",
        "Remove two charge counters from this artifact: It deals 3 damage to any target.",
    ],
    [
        "{3}, {T}, Pay 2 life: Draw a card.",
        "{2}, {T}, Pay 2 life: Draw a card.",
        "{1}, {T}, Pay 2 life: Draw a card.",
        "{1}, Pay 2 life: Draw a card.",
    ],
];

pub(super) fn ability_text(token: &TokenDef, index: u32) -> Option<&'static str> {
    for (family, tiers) in LIFE_RELICS.iter().enumerate() {
        if let Some(tier) = tiers
            .iter()
            .position(|candidate| std::ptr::eq(token, *candidate))
        {
            return match (family, index) {
                (2, 0) => Some("Whenever you gain life, put a charge counter on this artifact."),
                (2, 1) | (0 | 1 | 3, 0) => Some(TEXT[family][tier]),
                _ => None,
            };
        }
    }
    None
}
