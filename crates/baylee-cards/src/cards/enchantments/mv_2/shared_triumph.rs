//! Shared Triumph — {1}{W} — Enchantment
//! Oracle: As this enchantment enters, choose a creature type.
//! Oracle: Creatures of the chosen type get +1/+1.
//! Set: ONS #53 — Onslaught | Scryfall ID: 0d07ebe6-76cf-4345-b59b-9954496c44d0 | Oracle ID: 64d022ab-5222-4a7a-ab8a-89749560f666
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHARED_TRIUMPH,
    oracle_id = "64d022ab-5222-4a7a-ab8a-89749560f666",
    scryfall_id = "0d07ebe6-76cf-4345-b59b-9954496c44d0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Shared Triumph",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
