//! Original token-workshop identities and complete table-facing rules sentences.
#[allow(clippy::wildcard_imports)]
use super::*;

/// Seed Cradle, Swarm Banner, Harvest Wheel, Root Reservoir; Common through Mythic.
pub const GROWTH_RELICS: [[&TokenDef; 4]; 4] = [
    [
        &TOWER_CRADLE_COMMON,
        &TOWER_CRADLE_UNCOMMON,
        &TOWER_CRADLE_RARE,
        &TOWER_CRADLE_MYTHIC,
    ],
    [
        &TOWER_BANNER_COMMON,
        &TOWER_BANNER_UNCOMMON,
        &TOWER_BANNER_RARE,
        &TOWER_BANNER_MYTHIC,
    ],
    [
        &TOWER_HARVEST_COMMON,
        &TOWER_HARVEST_UNCOMMON,
        &TOWER_HARVEST_RARE,
        &TOWER_HARVEST_MYTHIC,
    ],
    [
        &TOWER_ROOTS_COMMON,
        &TOWER_ROOTS_UNCOMMON,
        &TOWER_ROOTS_RARE,
        &TOWER_ROOTS_MYTHIC,
    ],
];

const TEXT: [[&str; 4]; 4] = [
    [
        "Whenever you gain life, you may pay {3}. If you do, create a 1/1 green Plant creature token.",
        "Whenever you gain life, you may pay {2}. If you do, create a 1/1 green Plant creature token.",
        "Whenever you gain life, you may pay {1}. If you do, create a 1/1 green Plant creature token.",
        "Whenever you gain life, you may pay {1}. If you do, create two 1/1 green Plant creature tokens.",
    ],
    [
        "Creature tokens you control get +1/+0.",
        "Creature tokens you control get +1/+1.",
        "Creature tokens you control get +2/+1.",
        "Creature tokens you control get +2/+2.",
    ],
    [
        "{T}, Remove three charge counters from this artifact: Add {C}{C}.",
        "{T}, Remove two charge counters from this artifact: Add {C}{C}.",
        "{T}, Remove a charge counter from this artifact: Add {C}{C}.",
        "Remove a charge counter from this artifact: Add {C}{C}{C}.",
    ],
    [
        "{T}, Remove three charge counters from this artifact: Add {G}.",
        "{T}, Remove two charge counters from this artifact: Add {G}.",
        "Remove two charge counters from this artifact: Add {G}.",
        "Remove a charge counter from this artifact: Add {G}.",
    ],
];

pub(super) fn ability_text(token: &TokenDef, index: u32) -> Option<&'static str> {
    for (family, tiers) in GROWTH_RELICS.iter().enumerate() {
        if let Some(tier) = tiers
            .iter()
            .position(|candidate| std::ptr::eq(token, *candidate))
        {
            return match (family, tier, index) {
                (2, _, 0) => Some(
                    "Whenever a creature token you control dies, put a charge counter on this artifact.",
                ),
                (3, _, 0) => Some(
                    "Whenever a creature token you control enters, put a charge counter on this artifact.",
                ),
                (1, 3, 1) => Some("Creature tokens you control have trample."),
                (0 | 1, _, 0) | (2 | 3, _, 1) => Some(TEXT[family][tier]),
                _ => None,
            };
        }
    }
    None
}
