//! Vodalian Mystic — {1}{U} — Creature — Merfolk Wizard
//! Oracle: {T}: Target instant or sorcery spell becomes the color of your choice.
//! Set: APC #33 — Apocalypse | Scryfall ID: 81ec203a-067e-4360-9b4d-2d67db472aab | Oracle ID: c7feecf0-5229-4c12-806a-16c9ab38e147
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VODALIAN_MYSTIC,
    oracle_id = "c7feecf0-5229-4c12-806a-16c9ab38e147",
    scryfall_id = "81ec203a-067e-4360-9b4d-2d67db472aab",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Vodalian Mystic",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
