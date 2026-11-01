//! Desolation — {1}{B}{B} — Enchantment
//! Oracle: At the beginning of each end step, each player who tapped a land for mana this turn sacrifices a land of their choice. This enchantment deals 2 damage to each player who sacrificed a Plains this way.
//! Set: VIS #58 — Visions | Scryfall ID: 3b186460-d2af-4912-ba19-95b2cb5f1639 | Oracle ID: a80c3af8-470e-4e95-9579-cfdf18fbdbfa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DESOLATION,
    oracle_id = "a80c3af8-470e-4e95-9579-cfdf18fbdbfa",
    scryfall_id = "3b186460-d2af-4912-ba19-95b2cb5f1639",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Desolation",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
