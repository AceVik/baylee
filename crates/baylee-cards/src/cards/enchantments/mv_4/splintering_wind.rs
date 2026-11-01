//! Splintering Wind — {2}{G}{G} — Enchantment
//! Oracle: {2}{G}: This enchantment deals 1 damage to target creature. Create a 1/1 green Splinter creature token. It has flying and "Cumulative upkeep {G}." When it leaves the battlefield, it deals 1 damage to you and each creature you control. (At the beginning of its controller's upkeep, that player puts an age counter on it, then sacrifices it unless they pay its upkeep cost for each age counter on it.)
//! Set: ALL #99 — Alliances | Scryfall ID: 0afa94e5-fef6-4f3a-9196-d7aa6dd841c2 | Oracle ID: 184f68a1-0c13-42f4-aee6-3ca744f6a30b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPLINTERING_WIND,
    oracle_id = "184f68a1-0c13-42f4-aee6-3ca744f6a30b",
    scryfall_id = "0afa94e5-fef6-4f3a-9196-d7aa6dd841c2",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Splintering Wind",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
