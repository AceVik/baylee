//! Forsaken Wastes — {2}{B} — World Enchantment
//! Oracle: Players can't gain life.
//! Oracle: At the beginning of each player's upkeep, that player loses 1 life.
//! Oracle: Whenever this enchantment becomes the target of a spell, that spell's controller loses 5 life.
//! Set: MIR #125 — Mirage | Scryfall ID: c9dbfc7c-164d-47b8-8f05-987864fca89b | Oracle ID: b9e61e68-9dc8-4295-95dc-dd66a0907c8c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORSAKEN_WASTES,
    oracle_id = "b9e61e68-9dc8-4295-95dc-dd66a0907c8c",
    scryfall_id = "c9dbfc7c-164d-47b8-8f05-987864fca89b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Forsaken Wastes",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
