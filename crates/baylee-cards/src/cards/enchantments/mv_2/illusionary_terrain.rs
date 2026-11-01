//! Illusionary Terrain — {U}{U} — Enchantment
//! Oracle: Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: As this enchantment enters, choose two basic land types.
//! Oracle: Basic lands of the first chosen type are the second chosen type.
//! Set: ICE #77 — Ice Age | Scryfall ID: 691f4a1b-4706-41aa-82da-ae920739f036 | Oracle ID: 17ae63fa-fd63-45e6-81c1-bf30851bf77c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ILLUSIONARY_TERRAIN,
    oracle_id = "17ae63fa-fd63-45e6-81c1-bf30851bf77c",
    scryfall_id = "691f4a1b-4706-41aa-82da-ae920739f036",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Illusionary Terrain",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
