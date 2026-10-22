//! Familiar Ground — {2}{G} — Enchantment
//! Oracle: Each creature you control can't be blocked by more than one creature.
//! Set: 7ED #243 — Seventh Edition | Scryfall ID: a26e990d-726c-4b3b-84b4-9c15a5c4be8c | Oracle ID: 3b92c4dc-2605-42f4-a374-c3be4de3279e
// NOT SUPPORTED: no "can't be blocked by more than one creature" blocker-count restriction per attacking creature exists in Modifier yet.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FAMILIAR_GROUND,
    oracle_id = "3b92c4dc-2605-42f4-a374-c3be4de3279e",
    scryfall_id = "a26e990d-726c-4b3b-84b4-9c15a5c4be8c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "plays as an enchantment on the battlefield; no 'can't be blocked by more than one creature' blocker-count restriction per attacking creature exists in Modifier yet",
    ),
    faces = &[face!(
        name = "Familiar Ground",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);
